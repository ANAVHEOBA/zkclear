  api/encrypted-intent-gateway/
  ├── Cargo.toml
  ├── .env.example
  └── src/
      ├── main.rs
      ├── app.rs
      ├── config/
      │   ├── mod.rs
      │   ├── environment.rs
      │   └── db.rs
      ├── service/
      │   ├── mod.rs
      │   ├── signature_service.rs
      │   ├── replay_service.rs
      │   ├── decrypt_service.rs
      │   ├── commitment_service.rs
      │   └── workflow_service.rs
      ├── module/
      │   └── encrypted_intent/
      │       ├── mod.rs
      │       ├── model.rs
      │       ├── schema.rs
      │       ├── crud.rs
      │       ├── controller.rs
      │       └── route.rs
      ├── infra/
      │   ├── mod.rs
      - bad signature
      - replay nonce/hash
      - expired timestamp


       Not implemented yet (next features):

  - Intent submit API route
  - Signature verification
  - Nonce/replay protection
  - Decrypt-in-confidential-boundary logic
  - Commitment/hash generation
  - Mongo/Redis persistence layer
  - Controller + CRUD for encrypted intents