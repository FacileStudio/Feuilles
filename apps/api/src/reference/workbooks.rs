use utoipa::openapi::path::HttpMethod::{self, Delete, Get, Patch, Post, Put};

use super::table::Route;
use super::table::Shape::None as Empty;
use super::table::Shape::{Cells, Create, Rename, Spaces, Workbook, Workbooks, Writes, Written};

const fn wb(method: HttpMethod, path: &'static str, summary: &'static str, description: &'static str) -> Route {
    Route {
        method,
        path,
        tag: "Workbooks",
        summary,
        description,
        body: Empty,
        answer: (200, Empty),
        query: &[],
        errors: &[401, 404],
    }
}

const VIEWER: &str = "Any member of the workbook's space, or the owner of a personal workbook.";
const EDITOR: &str = "A space member, admin or owner, or the owner of a personal workbook.";
const MANAGER: &str = "A space admin or owner, or the owner of a personal workbook. A space member gets 403.";
const LIST: &str = "Newest change first. No `space_id` is personal scope; a space the caller is not in is 404. \
    `sheets` is empty in lists.";
const SOCKET: &str = "WebSocket, binary frames: see docs/api.md. Refusals after an upgrade close with 1008 and reason \
    `unauthenticated` or `not_found`; without upgrade headers they answer 401 or 404.";

/// The space, workbook, cell and socket routes.
pub const ROUTES: &[Route] = &[
    Route {
        tag: "Spaces",
        answer: (200, Spaces),
        errors: &[401],
        ..wb(Get, "/api/spaces", "My spaces", "Roles are member, admin or owner.")
    },
    Route {
        query: &["space_id", "trashed"],
        answer: (200, Workbooks),
        ..wb(Get, "/api/workbooks", "List workbooks", LIST)
    },
    Route {
        body: Create,
        answer: (201, Workbook),
        errors: &[400, 401, 404],
        ..wb(Post, "/api/workbooks", "Create a workbook", "In personal scope or a space the caller is in.")
    },
    Route {
        answer: (201, Workbook),
        errors: &[400, 401, 404, 413],
        ..wb(
            Post,
            "/api/workbooks/import",
            "Import an .xlsx",
            "Multipart: `file` (.xlsx, at most 20 MiB) and `space_id`.",
        )
    },
    Route { answer: (200, Workbook), ..wb(Get, "/api/workbooks/{id}", "Get a workbook", VIEWER) },
    Route {
        body: Rename,
        answer: (200, Workbook),
        errors: &[400, 401, 403, 404],
        ..wb(Patch, "/api/workbooks/{id}", "Rename", EDITOR)
    },
    Route {
        answer: (204, Empty),
        errors: &[401, 403, 404],
        ..wb(Delete, "/api/workbooks/{id}", "Move to the trash", MANAGER)
    },
    Route {
        answer: (200, Workbook),
        errors: &[401, 403, 404],
        ..wb(Post, "/api/workbooks/{id}/restore", "Restore", MANAGER)
    },
    Route {
        answer: (204, Empty),
        errors: &[401, 403, 404, 412],
        ..wb(Delete, "/api/workbooks/{id}/permanent", "Delete for good", MANAGER)
    },
    Route { ..wb(Get, "/api/workbooks/{id}/export.xlsx", "Export .xlsx", VIEWER) },
    Route {
        query: &["sheet"],
        errors: &[401, 404, 413],
        ..wb(Get, "/api/workbooks/{id}/export.csv", "Export a sheet as CSV", VIEWER)
    },
    Route {
        query: &["sheet", "range"],
        answer: (200, Cells),
        errors: &[400, 401, 404],
        ..wb(Get, "/api/workbooks/{id}/cells", "Read cells", "At most 10 000 cells. Viewer access.")
    },
    Route {
        body: Writes,
        answer: (200, Written),
        errors: &[400, 401, 403, 404, 412, 413],
        ..wb(
            Put,
            "/api/workbooks/{id}/cells",
            "Write cells",
            "1 to 5000 inputs as one batch, broadcast as client 0. Trashed: 412.",
        )
    },
    Route {
        answer: (101, Empty),
        errors: &[400, 401, 403, 404],
        ..wb(Get, "/api/workbooks/{id}/ws", "Collaboration socket", SOCKET)
    },
];
