use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use ledist_pi::{AppState, Profile, web_router};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn profiles_endpoint_returns_registered_profile() {
    let profile =
        Profile::from_toml("[profile]\nid='e233'\nname='E233'\ncanvas_width=128\ncanvas_height=32")
            .unwrap();
    let app = web_router(Arc::new(AppState::new(vec![profile])));
    let response = app
        .oneshot(Request::get("/api/profiles").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn invalid_apply_leaves_display_state_unchanged() {
    let profile =
        Profile::from_toml("[profile]\nid='e233'\nname='E233'\ncanvas_width=128\ncanvas_height=32")
            .unwrap();
    let state = Arc::new(AppState::new(vec![profile]));
    let app = web_router(state.clone());
    let response = app
        .oneshot(
            Request::post("/api/display/apply")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"profile_id":"e233","brightness":101,"values":{},"program":"blank"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(state.current_state().is_none());
}
