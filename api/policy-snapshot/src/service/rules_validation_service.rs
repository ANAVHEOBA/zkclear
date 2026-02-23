use serde_json::Value;

pub fn validate_rules_bundle(rules: &Value) -> Result<(), String> {
    let obj = rules
        .as_object()
        .ok_or_else(|| "rules must be a JSON object".to_string())?;

    let limits = obj
        .get("limits")
        .and_then(Value::as_object)
        .ok_or_else(|| "rules.limits object is required".to_string())?;
    let max_notional = limits
        .get("max_notional")
        .and_then(Value::as_i64)
        .ok_or_else(|| "rules.limits.max_notional integer is required".to_string())?;
    let min_notional = limits
        .get("min_notional")
        .and_then(Value::as_i64)
        .ok_or_else(|| "rules.limits.min_notional integer is required".to_string())?;
    if min_notional <= 0 || max_notional <= 0 || min_notional > max_notional {
        return Err("rules.limits values are invalid".to_string());
    }

    let countries = obj
        .get("countries")
        .and_then(Value::as_array)
        .ok_or_else(|| "rules.countries array is required".to_string())?;
    if countries.is_empty() || countries.iter().any(|v| v.as_str().is_none()) {
        return Err("rules.countries must be a non-empty array of strings".to_string());
    }

    let thresholds = obj
        .get("thresholds")
        .and_then(Value::as_object)
        .ok_or_else(|| "rules.thresholds object is required".to_string())?;
    let fail_conf = thresholds
        .get("fail_confidence")
        .and_then(Value::as_i64)
        .ok_or_else(|| "rules.thresholds.fail_confidence integer is required".to_string())?;
    let review_conf = thresholds
        .get("review_confidence")
        .and_then(Value::as_i64)
        .ok_or_else(|| "rules.thresholds.review_confidence integer is required".to_string())?;
    if !(0..=100).contains(&review_conf)
        || !(0..=100).contains(&fail_conf)
        || review_conf > fail_conf
    {
        return Err("rules.thresholds values are invalid".to_string());
    }

    Ok(())
}
