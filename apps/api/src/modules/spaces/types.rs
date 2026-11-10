use serde::Serialize;

/// One space the caller belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow, utoipa::ToSchema, schemars::JsonSchema)]
pub struct SpaceItem {
    /// Registre's id. Send it as `space_id`.
    pub id: String,
    /// The display name.
    pub name: String,
    /// `member`, `admin` or `owner`.
    pub role: String,
}

/// `GET /spaces` answer.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SpaceList {
    /// By name.
    pub spaces: Vec<SpaceItem>,
}
