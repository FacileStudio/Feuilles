use utoipa::PartialSchema;
use utoipa::openapi::path::{HttpMethod, OperationBuilder, ParameterBuilder, ParameterIn};
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::schema::{Object, Type};
use utoipa::openapi::{Content, PathItem, RefOr, Required, ResponseBuilder, Schema};

/// A JSON body the reference describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// No body.
    None,
    /// An object with no declared schema.
    Object,
    /// `Me`.
    Me,
    /// `SessionAnswer`.
    Session,
    /// `VersionAnswer`.
    Version,
    /// `SpaceList`.
    Spaces,
    /// `WorkbookList`.
    Workbooks,
    /// `Workbook`.
    Workbook,
    /// `CreateWorkbook`.
    Create,
    /// `RenameWorkbook`.
    Rename,
    /// `CellsAnswer`.
    Cells,
    /// `WriteCells`.
    Writes,
    /// `WriteAnswer`.
    Written,
}

impl Shape {
    fn schema(self) -> Option<RefOr<Schema>> {
        use crate::modules::workbooks::types as wb;
        Some(match self {
            Shape::None => return None,
            Shape::Object => RefOr::T(Schema::Object(Object::with_type(Type::Object))),
            Shape::Me => crate::auth::Me::schema(),
            Shape::Session => crate::auth::SessionAnswer::schema(),
            Shape::Version => crate::version::VersionAnswer::schema(),
            Shape::Spaces => crate::modules::spaces::types::SpaceList::schema(),
            Shape::Workbooks => wb::WorkbookList::schema(),
            Shape::Workbook => wb::Workbook::schema(),
            Shape::Create => wb::CreateWorkbook::schema(),
            Shape::Rename => wb::RenameWorkbook::schema(),
            Shape::Cells => wb::CellsAnswer::schema(),
            Shape::Writes => wb::WriteCells::schema(),
            Shape::Written => wb::WriteAnswer::schema(),
        })
    }
}

/// One documented route.
pub struct Route {
    /// The verb.
    pub method: HttpMethod,
    /// The full path, `{id}` style.
    pub path: &'static str,
    /// The group in the reference.
    pub tag: &'static str,
    /// One line.
    pub summary: &'static str,
    /// Who may call it, and what it does beyond the summary.
    pub description: &'static str,
    /// The JSON body.
    pub body: Shape,
    /// The success status and its JSON body.
    pub answer: (u16, Shape),
    /// The query parameters.
    pub query: &'static [&'static str],
    /// The error statuses it answers.
    pub errors: &'static [u16],
}

fn reason(status: u16) -> &'static str {
    match status {
        200 | 201 | 202 | 204 => "Success",
        302 => "Redirect",
        400 => "invalid_argument: a bad body, query or file",
        401 => "unauthenticated: no valid session or token",
        403 => "permission_denied: the role does not allow it, or a cookie write lacks X-Facile-CSRF",
        404 => "not_found: no such workbook, or no access to it",
        405 => "Method not allowed",
        412 => "failed_precondition: the workbook is not in the right state",
        413 => "resource_exhausted: the body or file is too large",
        _ => "internal or unavailable",
    }
}

impl Route {
    /// The OpenAPI path item.
    pub fn item(&self) -> PathItem {
        let json = |schema: RefOr<Schema>| Content::new(Some(schema));
        let success = ResponseBuilder::new().description(reason(self.answer.0));
        let success = match self.answer.1.schema() {
            Some(schema) => success.content("application/json", json(schema)),
            None => success,
        };
        let mut operation = OperationBuilder::new()
            .tag(self.tag)
            .summary(Some(self.summary))
            .description(Some(self.description))
            .response(self.answer.0.to_string(), success.build());
        for status in self.errors {
            operation =
                operation.response(status.to_string(), ResponseBuilder::new().description(reason(*status)).build());
        }
        for name in self.query {
            operation = operation.parameter(
                ParameterBuilder::new().name(*name).parameter_in(ParameterIn::Query).required(Required::False),
            );
        }
        if self.path.contains("{id}") {
            operation = operation
                .parameter(ParameterBuilder::new().name("id").parameter_in(ParameterIn::Path).required(Required::True));
        }
        if let Some(body) = self.body.schema() {
            operation =
                operation.request_body(Some(RequestBodyBuilder::new().content("application/json", json(body)).build()));
        }
        PathItem::new(self.method.clone(), operation)
    }
}
