use super::error::AppError;
use super::model::EncryptedIntent;
use super::schema::{SubmitIntentRequest, SubmitIntentResponse};
use crate::app::AppState;
use crate::service::commitment_service::compute_commitment;
use crate::service::decrypt_service::decrypt_intent;
use crate::service::signature_service::verify_signature;
use crate::service::workflow_service::generate_workflow_run_id;
use mongodb::Collection;
use redis::Script;
use std::time::{SystemTime, UNIX_EPOCH};

const INTENTS_COLLECTION: &str = "encrypted_intents";
const REPLAY_TTL_SECONDS: u64 = 60 * 60 * 24;

pub async fn submit_intent(
    state: &AppState,
    req: SubmitIntentRequest,
    forced_workflow_run_id: Option<String>,
) -> Result<SubmitIntentResponse, AppError> {
    validate_request(state, &req)?;

    verify_signature(
        &req.encrypted_payload,
        &req.nonce,
        req.timestamp,
        &req.signature,
        &req.signer_public_key,
    )
    .map_err(|e| AppError::bad_request("BAD_SIGNATURE", e))?;
    decrypt_intent(&req.encrypted_payload)
        .map_err(|e| AppError::bad_request("DECRYPT_FAILED", e))?;

    let workflow_run_id = forced_workflow_run_id
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(generate_workflow_run_id);
    let commitment_hash = compute_commitment(
        &req.encrypted_payload,
        &req.nonce,
        req.timestamp,
        &req.signer_public_key,
    );
    reserve_replay_keys(state, &req.nonce, &commitment_hash).await?;
    let intent_id = format!("intent-{}", &commitment_hash[..16]);

    let intent = EncryptedIntent {
        intent_id: intent_id.clone(),
        workflow_run_id: workflow_run_id.clone(),
        encrypted_payload: req.encrypted_payload,
        commitment_hash,
        signer_public_key: req.signer_public_key,
        nonce: req.nonce,
        timestamp: req.timestamp,
    };
    insert_intent(state, &intent).await?;

    Ok(SubmitIntentResponse {
        workflow_run_id,
        intent_ids: vec![intent_id],
        commitment_hashes: vec![intent.commitment_hash.clone()],
        accepted: true,
        error_code: None,
        reason: "accepted".to_string(),
    })
}

fn validate_request(state: &AppState, req: &SubmitIntentRequest) -> Result<(), AppError> {
    if req.encrypted_payload.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_PAYLOAD",
            "encrypted_payload is required",
        ));
    }
    if req.signature.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_SIGNATURE",
            "signature is required",
        ));
    }
    if req.signer_public_key.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_SIGNER",
            "signer_public_key is required",
        ));
    }
    if req.nonce.trim().is_empty() {
        return Err(AppError::bad_request("INVALID_NONCE", "nonce is required"));
    }
    if req.timestamp <= 0 {
        return Err(AppError::bad_request(
            "INVALID_TIMESTAMP",
            "timestamp must be a positive unix epoch",
        ));
    }

    let now = now_unix()?;
    let max_age = state.config.intent_max_age_seconds;
    if req.timestamp < now - max_age {
        return Err(AppError::bad_request(
            "INTENT_EXPIRED",
            "intent timestamp is older than allowed window",
        ));
    }
    let max_future_skew = state.config.intent_max_future_skew_seconds;
    if req.timestamp > now + max_future_skew {
        return Err(AppError::bad_request(
            "TIMESTAMP_IN_FUTURE",
            "intent timestamp exceeds future skew allowance",
        ));
    }
    Ok(())
}

fn now_unix() -> Result<i64, AppError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal("CLOCK_ERROR", format!("clock error: {e}")))?;
    Ok(elapsed.as_secs() as i64)
}

fn validate_intent_model(intent: &EncryptedIntent) -> Result<(), AppError> {
    if intent.intent_id.is_empty()
        || intent.workflow_run_id.is_empty()
        || intent.encrypted_payload.is_empty()
        || intent.commitment_hash.is_empty()
        || intent.signer_public_key.is_empty()
        || intent.nonce.is_empty()
        || intent.timestamp <= 0
    {
        return Err(AppError::bad_request("INVALID_INTENT", "invalid intent"));
    }
    Ok(())
}

async fn insert_intent(state: &AppState, intent: &EncryptedIntent) -> Result<(), AppError> {
    validate_intent_model(intent)?;

    let collection: Collection<EncryptedIntent> =
        state.infra.mongo_db.collection(INTENTS_COLLECTION);
    collection.insert_one(intent).await.map_err(|e| {
        AppError::internal("PERSISTENCE_ERROR", format!("mongodb insert failed: {e}"))
    })?;
    Ok(())
}

async fn reserve_replay_keys(
    state: &AppState,
    nonce: &str,
    commitment_hash: &str,
) -> Result<(), AppError> {
    let mut conn = state
        .infra
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::internal("REDIS_ERROR", format!("redis connection failed: {e}")))?;

    let nonce_key = format!("replay:nonce:{nonce}");
    let hash_key = format!("replay:hash:{commitment_hash}");

    let reserve_script = Script::new(
        r#"
        local nonceKey = KEYS[1]
        local hashKey = KEYS[2]
        local ttl = tonumber(ARGV[1])
        if redis.call('EXISTS', nonceKey) == 1 then
            return 1
        end
        if redis.call('EXISTS', hashKey) == 1 then
            return 2
        end
        redis.call('SET', nonceKey, '1', 'EX', ttl, 'NX')
        redis.call('SET', hashKey, '1', 'EX', ttl, 'NX')
        return 0
    "#,
    );

    let replay_status: i32 = reserve_script
        .key(&nonce_key)
        .key(&hash_key)
        .arg(REPLAY_TTL_SECONDS as i64)
        .invoke_async(&mut conn)
        .await
        .map_err(|e| {
            AppError::internal("REDIS_ERROR", format!("redis replay script failed: {e}"))
        })?;

    if replay_status == 1 {
        return Err(AppError::bad_request(
            "REPLAY_NONCE",
            "replay detected: nonce already used",
        ));
    }
    if replay_status == 2 {
        return Err(AppError::bad_request(
            "REPLAY_HASH",
            "replay detected: intent hash already used",
        ));
    }
    if replay_status != 0 {
        return Err(AppError::internal(
            "REDIS_ERROR",
            "redis replay script returned unknown status",
        ));
    }

    Ok(())
}
