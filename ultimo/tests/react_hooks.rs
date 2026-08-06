//! Golden-file test for generated TanStack Query React hooks.
//! Run with: cargo test -p ultimo --features client-gen --test react_hooks

#![cfg(feature = "client-gen")]

use ultimo::rpc::{RpcMode, RpcRegistry, TS};

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct GetUserInput {
    id: u32,
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct CreateUserInput {
    name: String,
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
struct User {
    id: u32,
    name: String,
}

fn registry() -> RpcRegistry {
    let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);
    rpc.query("getUser", |input: GetUserInput| async move {
        Ok(User {
            id: input.id,
            name: "x".into(),
        })
    });
    rpc.mutation("createUser", |input: CreateUserInput| async move {
        Ok(User {
            id: 1,
            name: input.name,
        })
    });
    rpc
}

#[test]
fn hooks_module_has_provider_query_and_mutation() {
    let hooks = registry().generate_react_hooks();

    // Imports the peer dep + the generated client (not named types).
    assert!(
        hooks.contains("@tanstack/react-query"),
        "missing tanstack import:\n{hooks}"
    );
    assert!(
        hooks.contains("import { UltimoRpcClient } from './client'"),
        "missing client import:\n{hooks}"
    );

    // Context injection.
    assert!(
        hooks.contains("export function UltimoProvider"),
        "missing provider:\n{hooks}"
    );
    assert!(
        hooks.contains("must be used within <UltimoProvider>"),
        "missing client guard:\n{hooks}"
    );

    // Query hook uses useQuery with a stable key and calls the client method.
    assert!(
        hooks.contains("export function useGetUser("),
        "missing query hook:\n{hooks}"
    );
    assert!(
        hooks.contains("return useQuery({"),
        "query must use useQuery:\n{hooks}"
    );
    assert!(
        hooks.contains("queryKey: queryKeys.getUser(input)"),
        "missing query key:\n{hooks}"
    );
    assert!(
        hooks.contains("queryFn: () => client.getUser(input)"),
        "missing query fn:\n{hooks}"
    );

    // Types derived from the client signature, not imported names.
    assert!(
        hooks.contains("Parameters<UltimoRpcClient['getUser']>[0]"),
        "input type not derived from client:\n{hooks}"
    );
    assert!(
        hooks.contains("Awaited<ReturnType<UltimoRpcClient['getUser']>>"),
        "output type not derived from client:\n{hooks}"
    );

    // Mutation hook uses useMutation.
    assert!(
        hooks.contains("export function useCreateUser("),
        "missing mutation hook:\n{hooks}"
    );
    assert!(
        hooks.contains("return useMutation({"),
        "mutation must use useMutation:\n{hooks}"
    );
    assert!(
        hooks.contains("mutationFn: (input:") && hooks.contains("client.createUser(input)"),
        "missing mutation fn:\n{hooks}"
    );

    // queryKeys covers the query but NOT the mutation.
    assert!(
        hooks.contains("getUser: (input:"),
        "missing queryKeys.getUser:\n{hooks}"
    );
    assert!(
        !hooks.contains("createUser: (input:"),
        "mutation must not appear in queryKeys:\n{hooks}"
    );
}

#[test]
fn generate_react_hooks_file_writes_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hooks.ts");
    registry()
        .generate_react_hooks_file(path.to_str().unwrap())
        .expect("writes file");
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("export function useGetUser("));
}
