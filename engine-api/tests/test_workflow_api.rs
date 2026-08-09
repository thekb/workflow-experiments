mod utilities;

use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use engine_api::apis::common::CursorSigner;
use engine_api::apis::middleware::UserContext;
use engine_api::apis::router::get_router;
use engine_api::{
    apis::config::AppState,
    entities::config::{
        workflow::Model as Workflow, workflow_version::Model as WorkflowVersion,
    },
    service::WorkflowService,
};

use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
use utilities::{PostgresConnection, start_postgres};
use uuid::Uuid;

const TENANT_ID: &str = "db88d556-9e21-45d1-b5b2-c9c923cd182c";

async fn test_app() -> Result<(Router, PostgresConnection), String> {
    let postgres = start_postgres().await?;
    let db = postgres.connection();
    db.get_schema_registry("engine_api::entities::*")
        .sync(db)
        .await
        .map_err(|error| format!("schema sync failed: {error}"))?;

    let state = Arc::new(AppState {
        workflows: WorkflowService::new(db.clone()),
        cursor_signer: CursorSigner::new(vec![42; 32]),
    });
    let app = get_router(state).layer(Extension(UserContext {
        tenant_id: Uuid::parse_str(TENANT_ID).unwrap(),
    }));
    Ok((app, postgres))
}

fn workflow_body(name: &str) -> Value {
    json!({"name": name, "config": {
        "steps": [{"name": "step-1", "config": {"Http": {
            "method": "GET", "url": "https://example.com", "body": null
        }}, "depends_on": []}], "max_retries": 1
    }})
}

fn post_json(uri: &str, body: Value, key: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header(CONTENT_TYPE, "application/json");
    if let Some(key) = key {
        builder = builder.header("X-IDEMPOTENCY-KEY", key);
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

async fn response_json(
    response: axum::response::Response,
) -> Result<Value, String> {
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

#[tokio::test]
async fn create_and_get_workflow() -> Result<(), String> {
    let (app, _postgres) = test_app().await?;
    let response = app
        .clone()
        .oneshot(post_json(
            "/workflows",
            workflow_body("example"),
            Some("create-example"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Workflow =
        serde_json::from_value(response_json(response).await?)
            .map_err(|error| error.to_string())?;
    assert_eq!(created.name, "example");
    assert_eq!(created.current_version, 1);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/workflows/{}", created.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let fetched: Workflow =
        serde_json::from_value(response_json(response).await?)
            .map_err(|error| error.to_string())?;
    assert_eq!(fetched.id, created.id);
    Ok(())
}

#[tokio::test]
async fn create_workflow_requires_idempotency_key() -> Result<(), String> {
    let (app, _postgres) = test_app().await?;
    let response = app
        .oneshot(post_json("/workflows", workflow_body("missing-key"), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn reused_idempotency_key_with_different_body_conflicts()
-> Result<(), String> {
    let (app, _postgres) = test_app().await?;
    let first = app
        .clone()
        .oneshot(post_json(
            "/workflows",
            workflow_body("first"),
            Some("same-key"),
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::CREATED);
    let second = app
        .oneshot(post_json(
            "/workflows",
            workflow_body("second"),
            Some("same-key"),
        ))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::CONFLICT);
    Ok(())
}

#[tokio::test]
async fn create_workflow_version_and_reject_stale_version() -> Result<(), String>
{
    let (app, _postgres) = test_app().await?;
    let response = app
        .clone()
        .oneshot(post_json(
            "/workflows",
            workflow_body("versioned"),
            Some("create-versioned"),
        ))
        .await
        .unwrap();
    let workflow: Workflow =
        serde_json::from_value(response_json(response).await?)
            .map_err(|error| error.to_string())?;
    let version_body = json!({"expected_current_version": 1, "config": {
        "steps": [], "max_retries": 2
    }});
    let response = app
        .clone()
        .oneshot(post_json(
            &format!("/workflows/{}/versions", workflow.id),
            version_body.clone(),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let version: WorkflowVersion =
        serde_json::from_value(response_json(response).await?)
            .map_err(|error| error.to_string())?;
    assert_eq!(version.version, 2);
    let stale = app
        .oneshot(post_json(
            &format!("/workflows/{}/versions", workflow.id),
            version_body,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    Ok(())
}
