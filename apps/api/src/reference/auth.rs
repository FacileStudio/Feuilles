use utoipa::openapi::path::HttpMethod::{Get, Post};

use super::table::Route;
use super::table::Shape::None as Empty;
use super::table::Shape::{Me, Object, Session, Version};

const fn auth(method: utoipa::openapi::path::HttpMethod, path: &'static str, summary: &'static str) -> Route {
    Route {
        method,
        path,
        tag: "Auth",
        summary,
        description: "",
        body: Empty,
        answer: (200, Empty),
        query: &[],
        errors: &[],
    }
}

const OWN: &str = "The person's own browser session only: a bearer token, an API session or a delegated agent gets 403. \
    404 when MCP is off.";

/// porte's routes, the Registre nudges, the MCP confirm page's API and `/version`.
pub const ROUTES: &[Route] = &[
    Route {
        answer: (200, Object),
        description: "`{\"sso_only\":true,\"oidc_enabled\"}`.",
        ..auth(Get, "/api/auth/config", "Sign-in options")
    },
    Route {
        answer: (302, Empty),
        description: "Mounted with OIDC. Redirects to Registre.",
        ..auth(Get, "/api/auth/oidc", "Start sign-in")
    },
    Route {
        answer: (302, Empty),
        description: "Mounted with OIDC. Sets the session cookie.",
        ..auth(Get, "/api/auth/oidc/callback", "Finish sign-in")
    },
    Route {
        body: Object,
        answer: (200, Object),
        errors: &[400, 401],
        description: "Mounted with OIDC. `{\"code\"}` to `{\"user_id\",\"token\"}` for a CLI.",
        ..auth(Post, "/api/auth/oidc/exchange", "Trade a CLI code")
    },
    Route {
        answer: (200, Object),
        errors: &[401],
        description: "Mounted with OIDC. `{\"synced\"}`.",
        ..auth(Post, "/api/auth/sync-profile", "Refresh the profile")
    },
    Route {
        answer: (200, Object),
        errors: &[400, 401, 503],
        description: "Mounted with OIDC. Form `logout_token`, from Registre. 503 when Registre's keys cannot be fetched.",
        ..auth(Post, "/api/auth/backchannel-logout", "Back-channel logout")
    },
    Route {
        answer: (200, Object),
        errors: &[403],
        description: "`{\"logged_out\":true}`.",
        ..auth(Post, "/api/auth/logout", "Sign out")
    },
    Route { answer: (200, Me), errors: &[401], ..auth(Get, "/api/auth/me", "The signed-in account") },
    Route {
        answer: (200, Session),
        description: "Always 200. `{\"user\":{\"id\",\"email\",\"name\",\"avatar_url\"?},\"bulle_url\"}`, `user` null \
            without a session. `bulle_url` is the origin of the Bulle sidebar.",
        ..auth(Get, "/api/auth/session", "The account, or null")
    },
    Route {
        tag: "Registre",
        answer: (202, Empty),
        errors: &[405],
        description: "Empty body. Wakes the directory sync.",
        ..auth(Post, "/api/registre/sync", "Directory nudge")
    },
    Route {
        tag: "Registre",
        answer: (202, Empty),
        errors: &[400, 405],
        description: "A signed user-deleted notice. Other methods answer 405.",
        ..auth(Post, "/api/registre/deletions", "Account deletion push")
    },
    Route {
        tag: "MCP",
        answer: (200, Object),
        errors: &[401, 403],
        description: OWN,
        ..auth(Get, "/api/mcp/actions", "My pending agent actions")
    },
    Route {
        tag: "MCP",
        answer: (200, Object),
        errors: &[401, 403, 404, 412],
        description: OWN,
        ..auth(Post, "/api/mcp/actions/{id}/confirm", "Confirm an agent action")
    },
    Route {
        tag: "MCP",
        answer: (204, Empty),
        errors: &[401, 403, 404],
        description: OWN,
        ..auth(Post, "/api/mcp/actions/{id}/dismiss", "Dismiss an agent action")
    },
    Route {
        tag: "Meta",
        answer: (200, Version),
        description: "`engine` is the IronCalc version the browser engine must equal.",
        ..auth(Get, "/version", "Release and engine version")
    },
];

const TOOLS: &str = "Streamable HTTP, stateless. A Registre bearer token with `feuilles.read`, and `feuilles.write` for \
    every tool but the reads (`feuilles_list_workbooks`, `feuilles_get_workbook`, `feuilles_read_range`). \
    `feuilles_delete_sheet`, `feuilles_delete_rows`, `feuilles_delete_columns` and `feuilles_clear_range` ask the person \
    first: `input_required`, or `pending_confirmation` with a `confirm_url` to `/pending`. Never plan-gated.";

/// The MCP routes, mounted when `MCP_RESOURCE` is set.
pub const MCP: &[Route] = &[
    Route {
        tag: "MCP",
        body: Object,
        answer: (200, Object),
        errors: &[401, 403],
        description: TOOLS,
        ..auth(Post, "/mcp", "MCP endpoint")
    },
    Route {
        tag: "MCP",
        answer: (200, Object),
        description: "RFC 9728.",
        ..auth(Get, "/.well-known/oauth-protected-resource/mcp", "Resource metadata")
    },
    Route {
        tag: "MCP",
        answer: (200, Object),
        description: "RFC 9728, bare path.",
        ..auth(Get, "/.well-known/oauth-protected-resource", "Resource metadata")
    },
];
