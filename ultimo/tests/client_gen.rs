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
        client.contains("export type User = "),
        "User decl must be exported:\n{client}"
    );
    assert!(client.contains("email: string"));
    assert!(client.contains("tags: Array<string>"));
    assert!(client.contains("nickname: string | null"));

    // No dangling/hardcoded interface.
    assert!(!client.contains("export interface User"));
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct Game {
    id: u32,
    home_team: String,
    away_team: String,
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct Empty {}

#[test]
fn bare_vec_return_type_still_exports_inner_struct() {
    let rpc = RpcRegistry::new();
    rpc.query("getGames", |_: Empty| async move {
        Ok(vec![Game {
            id: 1,
            home_team: "A".into(),
            away_team: "B".into(),
        }])
    });

    let client = rpc.generate_typescript_client();

    assert!(
        client.contains("Promise<Array<Game>>"),
        "signature missing:\n{client}"
    );
    assert!(
        client.contains("export type Game = "),
        "Game's own declaration must be emitted for a bare Vec<Game> root type:\n{client}"
    );
}

#[test]
fn bare_option_return_type_still_exports_inner_struct() {
    let rpc = RpcRegistry::new();
    rpc.query("maybeGetGame", |_: Empty| async move {
        Ok(Some(Game {
            id: 1,
            home_team: "A".into(),
            away_team: "B".into(),
        }))
    });

    let client = rpc.generate_typescript_client();

    assert!(
        client.contains("export type Game = "),
        "Game's own declaration must be emitted for a bare Option<Game> root type:\n{client}"
    );
}
