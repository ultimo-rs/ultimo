//! Golden-file test for derived TypeScript client generation.
//! Run with: cargo test -p ultimo --features client-gen --test client_gen

#![cfg(feature = "client-gen")]

use ultimo::rpc::{RpcMode, RpcRegistry, TS};

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct User {
    id: u32,
    name: String,
    email: String,
    tags: Vec<String>,
    nickname: Option<String>,
}

#[test]
fn rest_client_has_derived_types_and_signatures() {
    let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);
    rpc.mutation("createUser", |input: CreateUserInput| async move {
        Ok(User {
            id: 1,
            name: input.name,
            email: input.email,
            tags: vec![],
            nickname: None,
        })
    });

    let client = rpc.generate_typescript_client();

    // Signature uses the derived named types.
    assert!(
        client.contains("async createUser(params: CreateUserInput): Promise<User>"),
        "signature missing:\n{client}"
    );

    // Both interfaces are declared with their real shapes.
    assert!(
        client.contains("type CreateUserInput = "),
        "input decl missing:\n{client}"
    );
    assert!(
        client.contains("type User = "),
        "output decl missing:\n{client}"
    );
    assert!(client.contains("email: string"));
    assert!(client.contains("tags: Array<string>"));
    assert!(client.contains("nickname: string | null"));

    // No dangling/hardcoded interface.
    assert!(!client.contains("export interface User"));
}
