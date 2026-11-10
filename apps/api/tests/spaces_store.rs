mod common;

use api::auth::Members;

#[tokio::test]
async fn members_pass_the_porte_spaces_conformance_suite() {
    let Some(state) = common::state().await else { return };
    let pool = state.pool.clone();
    let result = porte::spaces::conformance::run(|| {
        let pool = pool.clone();
        async move {
            sqlx::query("DELETE FROM spaces").execute(&pool).await.expect("reset");
            Members::new(pool)
        }
    })
    .await;
    assert!(result.is_ok(), "{:#?}", result.unwrap_err());
}
