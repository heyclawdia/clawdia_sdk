mod archive;
mod dispatch;
mod legacy_office;
mod media;
mod ocr;
mod office;
mod pdf;
mod rendered;
mod sqlite;
mod summary;
mod text;
mod url_resource;

pub(super) use dispatch::{
    render_bounded_prefix_read, render_workspace_read, render_workspace_uri,
};
pub(super) use rendered::{RenderedRead, RenderedUriRead};

use dispatch::extraction_error;
use rendered::{TRUNCATION_GUIDANCE, add_truncation_guidance};
