use uuid::Uuid;

use axum::{
    extract::Request, http::StatusCode, middleware::Next, response::Response,
};

#[derive(Clone, Debug)]
pub struct UserContext {
    // pub id: Uuid,
    // pub name: String,
    pub tenant_id: Uuid,
}

pub async fn user_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let parsed_tenant_id =
        Uuid::parse_str("db88d556-9e21-45d1-b5b2-c9c923cd182c")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    request.extensions_mut().insert(UserContext {
        tenant_id: parsed_tenant_id,
    });
    let response = next.run(request).await;
    Ok(response)
}
