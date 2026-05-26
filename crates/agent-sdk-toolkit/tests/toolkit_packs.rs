use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use agent_sdk_core::{
    AgentId, AllowToolPolicy, PolicyKind, PolicyRef, ProviderRouteSnapshot, RunId, RuntimePackage,
    RuntimePackageBuilder, RuntimePackageId, SourceKind, SourceRef, ToolCallId,
    ToolExecutionContext, ToolExecutionCoordinator, ToolPackId, ToolPackKind,
    domain::ContentRef,
    effect::EffectTerminalStatus,
    tool_ports::{ToolCallRequest, ToolExecutorRegistry, ToolRegistrySnapshot, ToolRouter},
    tool_records::CanonicalToolName,
};
use agent_sdk_toolkit::{
    AsyncTool, BoundedWorkspace, InMemoryJsonArgumentStore, InMemoryResourceResolver,
    InMemoryToolkitContentStore, ResourceReaderExecutor, ResourceReaderRequest,
    ShellExecutionPolicy, ShellExecutor, ShellRequest, Tool, ToolDiscoveryExecutor,
    ToolDiscoveryIndex, ToolPackBuilder, ToolkitPackBundle, ToolkitToolExecutionMode,
    WorkspaceEditExecutor, WorkspaceEditOutput, WorkspaceEditRequest, WorkspaceFileKind,
    WorkspacePolicy, WorkspaceReadExecutor, WorkspaceReadOutput, WorkspaceReadRequest,
    WorkspaceSearchExecutor, WorkspaceSearchRequest, WorkspaceWriteExecutor, WorkspaceWriteMode,
    WorkspaceWriteOutput, WorkspaceWriteRequest,
};

#[test]
fn ergonomic_tool_wrappers_lower_to_package_routes_without_execution() {
    let read_tool = Tool::builder(
        "workspace_read",
        "executor.workspace_read.v1",
        "schema.workspace_read",
        permission_policy("policy.fs.read"),
    )
    .read_only()
    .build()
    .expect("read tool declaration builds");
    let async_write = AsyncTool::builder(
        "workspace_write",
        "executor.workspace_write.v1",
        "schema.workspace_write",
        permission_policy("policy.fs.write"),
    )
    .write_effect()
    .build_async()
    .expect("async write declaration builds");

    assert_eq!(read_tool.mode(), &ToolkitToolExecutionMode::Sync);
    assert_eq!(async_write.mode(), &ToolkitToolExecutionMode::Async);
    assert_eq!(async_write.snapshot().timeout_ms, 60_000);
    assert_eq!(async_write.snapshot().cancellation, "cooperative");

    let bundle = ToolPackBuilder::new(
        ToolPackId::new("toolpack.ergonomic"),
        ToolPackKind::External,
        "v1",
        source(),
    )
    .listen(read_tool)
    .listen_async(async_write)
    .build()
    .expect("ergonomic pack builds");
    let package = package_for_bundle(&bundle);
    let snapshot =
        ToolRegistrySnapshot::from_runtime_package(&package, bundle.routes.clone()).unwrap();

    assert_eq!(bundle.capabilities.len(), 2);
    assert_eq!(snapshot.routes.len(), 2);
    assert!(
        snapshot
            .routes
            .iter()
            .any(|route| route.canonical_tool_name.as_str() == "workspace_read")
    );
    assert!(
        snapshot
            .routes
            .iter()
            .any(|route| route.canonical_tool_name.as_str() == "workspace_write")
    );
}

#[test]
fn workspace_read_routes_through_tool_executor_and_returns_anchor_hashes() {
    let workspace_root = temp_workspace("read");
    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.max_file_bytes = 1024;
    let workspace = Arc::new(BoundedWorkspace::new(policy));
    let args = InMemoryJsonArgumentStore::default();
    let content = InMemoryToolkitContentStore::default();
    let bundle = WorkspaceReadExecutor::pack_bundle(
        source(),
        permission_policy("policy.fs.read"),
        workspace.policy(),
    )
    .expect("read pack");
    let executor = Arc::new(WorkspaceReadExecutor::new(
        workspace.clone(),
        args.clone(),
        content.clone(),
    ));
    let package = package_for_bundle(&bundle);
    let args_ref = ContentRef::new("content.args.read.1");
    args.insert(
        args_ref.clone(),
        &WorkspaceReadRequest {
            path: "notes.md".to_string(),
            max_bytes: None,
        },
    )
    .expect("args stored");

    let outcome = execute_tool(
        &package,
        &bundle,
        executor,
        "workspace_read",
        "tool.call.read.1",
        args_ref,
    );

    assert_eq!(outcome.record.result_content_refs.len(), 1);
    let output: WorkspaceReadOutput = content
        .get(&outcome.record.result_content_refs[0])
        .expect("read output stored behind content ref");
    assert_eq!(output.mime_type, "text/markdown; charset=utf-8");
    assert_eq!(output.detected.kind, WorkspaceFileKind::Markdown);
    assert!(!output.binary);
    assert!(output.content_hash.starts_with("sha256:"));
    assert!(!output.truncated);
    assert_eq!(output.anchors[0].line, 1);
    assert!(output.anchors[0].before_hash.starts_with("sha256:"));
}

#[test]
fn workspace_read_extracts_pdf_image_raw_docx_and_archive_shapes() {
    let workspace_root = temp_workspace("read-binary");
    install_reader_fixture(&workspace_root, "brief.pdf");
    install_reader_fixture(&workspace_root, "pixel.png");
    install_reader_fixture(&workspace_root, "photo.dng");
    install_reader_fixture(&workspace_root, "doc.docx");
    install_reader_fixture(&workspace_root, "bundle.zip");
    install_reader_fixture(&workspace_root, "huge.docx");
    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.max_file_bytes = 512 * 1024;
    let workspace = BoundedWorkspace::new(policy);

    let pdf = workspace
        .read(&WorkspaceReadRequest {
            path: "brief.pdf".to_string(),
            max_bytes: None,
        })
        .expect("read pdf text");
    assert_eq!(pdf.detected.kind, WorkspaceFileKind::Pdf);
    assert_eq!(pdf.mime_type, "application/pdf");
    assert!(pdf.binary);
    assert!(pdf.content.contains("Hello PDF text"));
    assert!(pdf.anchors.is_empty());
    assert_eq!(pdf.document.unwrap().page_count, Some(1));

    let image = workspace
        .read(&WorkspaceReadRequest {
            path: "pixel.png".to_string(),
            max_bytes: None,
        })
        .expect("read image metadata");
    assert_eq!(image.detected.kind, WorkspaceFileKind::Image);
    let image_media = image.media.unwrap();
    assert_eq!(image_media.width, Some(1));
    assert_eq!(image_media.height, Some(1));
    assert!(image.content.contains("image/png"));

    let raw = workspace
        .read(&WorkspaceReadRequest {
            path: "photo.dng".to_string(),
            max_bytes: None,
        })
        .expect("read raw metadata");
    assert_eq!(raw.detected.kind, WorkspaceFileKind::RawImage);
    let raw_media = raw.media.unwrap();
    assert_eq!(raw_media.width, Some(4032));
    assert_eq!(raw_media.height, Some(3024));

    let docx = workspace
        .read(&WorkspaceReadRequest {
            path: "doc.docx".to_string(),
            max_bytes: None,
        })
        .expect("read docx text");
    assert_eq!(docx.detected.kind, WorkspaceFileKind::OfficeDocument);
    assert!(docx.content.contains("Hello docx text"));

    let archive = workspace
        .read(&WorkspaceReadRequest {
            path: "bundle.zip".to_string(),
            max_bytes: None,
        })
        .expect("read zip listing");
    assert_eq!(archive.detected.kind, WorkspaceFileKind::Archive);
    assert!(archive.content.contains("a.txt"));
    assert_eq!(archive.archive.unwrap().entry_count, 1);

    let huge_doc = workspace
        .read(&WorkspaceReadRequest {
            path: "huge.docx".to_string(),
            max_bytes: None,
        })
        .expect("huge docx is bounded");
    assert!(
        huge_doc
            .warnings
            .iter()
            .any(|warning| warning.contains("exceeds limit"))
    );
}

#[test]
fn workspace_read_handles_extended_reader_fixtures() {
    let workspace_root = temp_workspace("read-extended");
    for name in [
        "sample.tar",
        "sample.tgz",
        "sample.txt.gz",
        "sample.sqlite",
        "ocr.png",
        "ocr.png.ocr.txt",
        "scanned.pdf",
        "scanned.pdf.ocr.txt",
        "photo-preview.dng",
        "photo-preview.dng.aae",
        "legacy.doc",
        "legacy.doc.txt",
        "legacy.xls",
        "legacy.xls.txt",
        "legacy.ppt",
        "legacy.ppt.txt",
    ] {
        install_reader_fixture(&workspace_root, name);
    }
    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.max_file_bytes = 512 * 1024;
    let workspace = BoundedWorkspace::new(policy);

    let tar = workspace
        .read(&WorkspaceReadRequest {
            path: "sample.tar".to_string(),
            max_bytes: None,
        })
        .expect("read tar listing");
    assert_eq!(tar.detected.kind, WorkspaceFileKind::Archive);
    assert!(tar.content.contains("safe.txt"));
    assert!(
        tar.warnings
            .iter()
            .any(|warning| warning.contains("unsafe path"))
    );

    let tgz = workspace
        .read(&WorkspaceReadRequest {
            path: "sample.tgz".to_string(),
            max_bytes: None,
        })
        .expect("read tgz listing");
    assert_eq!(tgz.detected.kind, WorkspaceFileKind::Archive);
    assert!(tgz.content.contains("tgz-safe.txt"));
    assert!(tgz.archive.as_ref().unwrap().parser.contains("tar+gzip"));

    let gzip = workspace
        .read(&WorkspaceReadRequest {
            path: "sample.txt.gz".to_string(),
            max_bytes: None,
        })
        .expect("read gzip text");
    assert_eq!(gzip.detected.kind, WorkspaceFileKind::Archive);
    assert!(gzip.content.contains("hello from gzip fixture"));
    assert_eq!(gzip.archive.as_ref().unwrap().entry_count, 1);

    let sqlite = workspace
        .read(&WorkspaceReadRequest {
            path: "sample.sqlite".to_string(),
            max_bytes: None,
        })
        .expect("read sqlite schema and samples");
    assert_eq!(sqlite.detected.kind, WorkspaceFileKind::SqliteDatabase);
    assert!(sqlite.content.contains("notes"));
    assert!(sqlite.content.contains("sqlite fixture body"));
    assert_eq!(sqlite.sqlite.as_ref().unwrap().table_count, 1);

    let image_ocr = workspace
        .read(&WorkspaceReadRequest {
            path: "ocr.png".to_string(),
            max_bytes: None,
        })
        .expect("read image OCR sidecar");
    assert_eq!(image_ocr.detected.kind, WorkspaceFileKind::Image);
    assert!(
        image_ocr
            .content
            .contains("OCR SIDE TEXT from image fixture")
    );
    assert!(image_ocr.document.as_ref().unwrap().ocr.is_some());

    let pdf_ocr = workspace
        .read(&WorkspaceReadRequest {
            path: "scanned.pdf".to_string(),
            max_bytes: None,
        })
        .expect("read pdf OCR sidecar");
    assert_eq!(pdf_ocr.detected.kind, WorkspaceFileKind::Pdf);
    assert!(pdf_ocr.content.contains("OCR SIDE TEXT from pdf fixture"));
    assert!(pdf_ocr.document.as_ref().unwrap().ocr.is_some());

    let raw = workspace
        .read(&WorkspaceReadRequest {
            path: "photo-preview.dng".to_string(),
            max_bytes: None,
        })
        .expect("read raw preview/sensor metadata");
    let media = raw.media.as_ref().unwrap();
    assert_eq!(raw.detected.kind, WorkspaceFileKind::RawImage);
    assert_eq!(media.width, Some(4));
    assert_eq!(media.height, Some(3));
    assert_eq!(media.embedded_previews.len(), 1);
    assert!(media.raw_sensor.as_ref().unwrap().decoded_pixels);
    assert!(media.apple_photos.is_some());

    for (path, expected) in [
        ("legacy.doc", "Legacy DOC sidecar text fixture"),
        ("legacy.xls", "Legacy XLS sidecar text fixture"),
        ("legacy.ppt", "Legacy PPT sidecar text fixture"),
    ] {
        let legacy = workspace
            .read(&WorkspaceReadRequest {
                path: path.to_string(),
                max_bytes: None,
            })
            .expect("read legacy office fallback");
        assert_eq!(legacy.detected.kind, WorkspaceFileKind::OfficeDocument);
        assert!(legacy.content.contains(expected));
        assert!(
            legacy
                .warnings
                .iter()
                .any(|warning| warning.contains("bounded fallback"))
        );
    }
}

#[test]
fn workspace_read_uri_and_malformed_fixtures_are_bounded() {
    let workspace_root = temp_workspace("read-malformed");
    for name in [
        "malformed.sqlite",
        "corrupt.png",
        "corrupt.pdf",
        "encrypted-placeholder.pdf",
        "corrupt.zip",
        "corrupt.tar",
    ] {
        install_reader_fixture(&workspace_root, name);
    }
    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.max_file_bytes = 512 * 1024;
    let workspace = BoundedWorkspace::new(policy);

    let data = workspace
        .read(&WorkspaceReadRequest {
            path: "data:text/plain;base64,SGVsbG8gZGF0YSBVUkk=".to_string(),
            max_bytes: None,
        })
        .expect("read data URL");
    assert_eq!(data.detected.kind, WorkspaceFileKind::UrlResource);
    assert!(data.content.contains("Hello data URI"));
    assert!(data.resource.is_some());

    for uri in ["resource://summary", "https://example.invalid/data.txt"] {
        let denied = workspace
            .read(&WorkspaceReadRequest {
                path: uri.to_string(),
                max_bytes: None,
            })
            .expect_err("external/resource URI fails closed");
        assert_eq!(denied.kind(), agent_sdk_core::AgentErrorKind::PolicyDenial);
        assert!(
            denied.context().message.contains("host resource resolver")
                || denied.context().message.contains("network policy")
        );
    }

    let malformed_sqlite = workspace
        .read(&WorkspaceReadRequest {
            path: "malformed.sqlite".to_string(),
            max_bytes: None,
        })
        .expect("malformed sqlite is bounded warning output");
    assert_eq!(
        malformed_sqlite.detected.kind,
        WorkspaceFileKind::SqliteDatabase
    );
    assert!(
        malformed_sqlite
            .warnings
            .iter()
            .any(|warning| warning.contains("SQLite") && warning.contains("failed"))
    );

    let corrupt_png = workspace
        .read(&WorkspaceReadRequest {
            path: "corrupt.png".to_string(),
            max_bytes: Some(64),
        })
        .expect("corrupt image reports warning");
    assert_eq!(corrupt_png.detected.kind, WorkspaceFileKind::Image);
    assert!(corrupt_png.binary);
    assert!(
        corrupt_png
            .warnings
            .iter()
            .any(|warning| warning.contains("decode failed"))
    );

    for path in ["corrupt.pdf", "encrypted-placeholder.pdf"] {
        let error = workspace
            .read(&WorkspaceReadRequest {
                path: path.to_string(),
                max_bytes: None,
            })
            .expect_err("bad pdf returns typed parser failure");
        assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::ToolFailure);
    }

    for path in ["corrupt.zip", "corrupt.tar"] {
        let archive = workspace
            .read(&WorkspaceReadRequest {
                path: path.to_string(),
                max_bytes: Some(128),
            })
            .expect("corrupt archive is warning output");
        assert_eq!(archive.detected.kind, WorkspaceFileKind::Archive);
        assert!(archive.binary);
        assert!(!archive.content.contains("PK\u{3}\u{4}"));
        assert!(!archive.warnings.is_empty());
    }
}

#[test]
fn workspace_read_truncation_matrix_is_explicit_and_guided() {
    let workspace_root = temp_workspace("read-truncation-matrix");
    for name in [
        "brief.pdf",
        "doc.docx",
        "ocr.png",
        "ocr.png.ocr.txt",
        "sample.tar",
        "sample.sqlite",
    ] {
        install_reader_fixture(&workspace_root, name);
    }
    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.max_file_bytes = 512 * 1024;
    policy.max_output_bytes = 16;
    let workspace = BoundedWorkspace::new(policy);

    for path in [
        "brief.pdf",
        "doc.docx",
        "ocr.png",
        "sample.tar",
        "sample.sqlite",
    ] {
        let output = workspace
            .read(&WorkspaceReadRequest {
                path: path.to_string(),
                max_bytes: Some(16),
            })
            .expect("tiny read remains bounded");
        assert!(
            output.content.len() <= 16,
            "{path} content should be bounded"
        );
        assert!(output.truncated, "{path} should report truncation");
        assert_has_truncation_guidance(&output);
        assert_no_raw_binary_prefix(&output);
    }

    let data = workspace
        .read(&WorkspaceReadRequest {
            path: "data:text/plain,ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWXYZ"
                .to_string(),
            max_bytes: Some(12),
        })
        .expect("data URL truncates with guidance");
    assert_eq!(data.detected.kind, WorkspaceFileKind::UrlResource);
    assert!(data.truncated);
    assert_has_truncation_guidance(&data);
}

#[test]
fn workspace_read_oversized_prefix_is_utf8_safe_and_downgrades_parsers() {
    let workspace_root = temp_workspace("read-oversized-prefix");
    for name in ["brief.pdf", "sample.tar", "sample.sqlite", "pixel.png"] {
        install_reader_fixture(&workspace_root, name);
    }
    fs::write(workspace_root.join("unicode.txt"), "hello ééééé").unwrap();
    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.max_file_bytes = 8;
    policy.max_output_bytes = 256;
    let workspace = BoundedWorkspace::new(policy);

    let unicode = workspace
        .read(&WorkspaceReadRequest {
            path: "unicode.txt".to_string(),
            max_bytes: None,
        })
        .expect("oversized UTF-8 prefix succeeds even at character boundary");
    assert!(unicode.truncated);
    assert!(unicode.content.contains("hello"));
    assert_has_truncation_guidance(&unicode);

    for path in ["brief.pdf", "sample.tar", "sample.sqlite", "pixel.png"] {
        let output = workspace
            .read(&WorkspaceReadRequest {
                path: path.to_string(),
                max_bytes: None,
            })
            .expect("oversized parser input downgrades to summary");
        assert!(output.truncated, "{path} should be truncated");
        assert!(
            output
                .content_summary
                .as_deref()
                .unwrap_or(&output.content)
                .contains("Parser adapters that require the whole file were not run"),
            "{path} should not run full parser over oversized input"
        );
        assert_has_truncation_guidance(&output);
        assert_no_raw_binary_prefix(&output);
    }
}

#[test]
fn workspace_read_scanned_pdf_without_ocr_sidecar_reports_ocr_need() {
    let workspace_root = temp_workspace("read-ocr-needed");
    install_reader_fixture(&workspace_root, "scanned.pdf");
    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.max_file_bytes = 512 * 1024;
    let workspace = BoundedWorkspace::new(policy);

    let output = workspace
        .read(&WorkspaceReadRequest {
            path: "scanned.pdf".to_string(),
            max_bytes: None,
        })
        .expect("scanned PDF without sidecar is a bounded warning read");
    assert_eq!(output.detected.kind, WorkspaceFileKind::Pdf);
    assert!(
        output
            .warnings
            .iter()
            .any(|warning| warning.contains("OCR may be required")
                || warning.contains("OCR sidecar"))
    );
    assert!(output.document.as_ref().unwrap().ocr.is_none());
}

#[test]
fn workspace_read_declared_text_with_binary_bytes_is_summarized() {
    let workspace_root = temp_workspace("read-hostile-text");
    fs::write(workspace_root.join("bad.txt"), b"hello\0binary").unwrap();
    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.max_file_bytes = 512 * 1024;
    let workspace = BoundedWorkspace::new(policy);

    let file = workspace
        .read(&WorkspaceReadRequest {
            path: "bad.txt".to_string(),
            max_bytes: None,
        })
        .expect("declared text with NUL is summarized as binary");
    assert_eq!(file.detected.kind, WorkspaceFileKind::Binary);
    assert!(file.binary);
    assert!(file.content.is_empty());
    assert!(!file.content.contains('\0'));

    let uri = workspace
        .read(&WorkspaceReadRequest {
            path: "data:text/plain;base64,AAECAw==".to_string(),
            max_bytes: None,
        })
        .expect("declared text data URL with NUL is summarized");
    assert_eq!(uri.detected.kind, WorkspaceFileKind::UrlResource);
    assert!(uri.binary);
    assert!(uri.content.contains("Binary URI content was not emitted"));
    assert!(!uri.content.contains('\0'));
}

#[test]
fn workspace_read_archive_decompression_caps_are_top_level_truncation() {
    let workspace_root = temp_workspace("read-archive-cap");
    for name in ["huge.txt.gz", "huge.bin.gz", "huge.tgz"] {
        install_reader_fixture(&workspace_root, name);
    }
    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.max_file_bytes = 512 * 1024;
    policy.max_output_bytes = 8 * 1024 * 1024;
    let workspace = BoundedWorkspace::new(policy);

    for path in ["huge.txt.gz", "huge.bin.gz", "huge.tgz"] {
        let output = workspace
            .read(&WorkspaceReadRequest {
                path: path.to_string(),
                max_bytes: Some(8 * 1024 * 1024),
            })
            .expect("huge compressed archive is capped");
        assert_eq!(output.detected.kind, WorkspaceFileKind::Archive);
        assert!(output.truncated, "{path} should be top-level truncated");
        assert!(output.archive.as_ref().unwrap().truncated);
        assert_has_truncation_guidance(&output);
        assert!(
            output
                .warnings
                .iter()
                .any(|warning| warning.contains("decompression hit the reader cap")),
            "{path} should report decompression cap"
        );
        assert_no_raw_binary_prefix(&output);
    }
}

#[test]
fn search_respects_match_limits_and_reports_regex_compile_error() {
    let workspace_root = temp_workspace("search");
    fs::write(
        workspace_root.join("image.png"),
        b"\x89PNG\r\n\x1a\nalpha should not be searched",
    )
    .unwrap();
    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.max_matches = 10;
    let workspace = BoundedWorkspace::new(policy);

    let output = workspace
        .search(&WorkspaceSearchRequest {
            pattern: "alpha".to_string(),
        })
        .expect("search succeeds");
    assert_eq!(output.matches.len(), 3);
    assert!(!output.matches.iter().any(|item| item.path == "image.png"));

    let mut limited_policy = WorkspacePolicy::new(&workspace_root);
    limited_policy.max_matches = 1;
    let limited_workspace = BoundedWorkspace::new(limited_policy);
    let limited_output = limited_workspace
        .search(&WorkspaceSearchRequest {
            pattern: "alpha".to_string(),
        })
        .expect("limited search succeeds");
    assert_eq!(limited_output.matches.len(), 1);
    assert!(limited_output.truncated);

    let error = workspace
        .search(&WorkspaceSearchRequest {
            pattern: "[".to_string(),
        })
        .expect_err("invalid regex reports compile error");
    assert!(error.context().message.contains("regex compile error"));
}

#[test]
fn edit_preview_does_not_write_and_apply_fails_on_stale_anchor() {
    let workspace_root = temp_workspace("edit");
    let workspace = Arc::new(BoundedWorkspace::new(WorkspacePolicy::new(&workspace_root)));
    let args = InMemoryJsonArgumentStore::default();
    let content = InMemoryToolkitContentStore::default();
    let bundle = WorkspaceEditExecutor::pack_bundle(
        source(),
        permission_policy("policy.fs.write"),
        workspace.policy(),
    )
    .expect("edit pack");
    let package = package_for_bundle(&bundle);
    let executor = Arc::new(WorkspaceEditExecutor::new(
        workspace.clone(),
        args.clone(),
        content.clone(),
    ));
    let read = workspace
        .read(&WorkspaceReadRequest {
            path: "notes.md".to_string(),
            max_bytes: None,
        })
        .expect("read fixture");
    let anchor = read.anchors[1].clone();

    let preview_args = ContentRef::new("content.args.edit.preview");
    args.insert(
        preview_args.clone(),
        &WorkspaceEditRequest {
            path: "notes.md".to_string(),
            anchor: anchor.clone(),
            replacement: "beta replaced".to_string(),
            preview_only: true,
            preview_hash: None,
        },
    )
    .unwrap();
    let preview_outcome = execute_tool(
        &package,
        &bundle,
        executor.clone(),
        "workspace_edit",
        "tool.call.edit.preview",
        preview_args,
    );
    let preview: WorkspaceEditOutput = content
        .get(&preview_outcome.record.result_content_refs[0])
        .expect("preview stored");
    assert!(!preview.applied);
    assert!(
        fs::read_to_string(workspace_root.join("notes.md"))
            .unwrap()
            .contains("beta two")
    );

    let missing_preview_hash_args = ContentRef::new("content.args.edit.missing_preview");
    args.insert(
        missing_preview_hash_args.clone(),
        &WorkspaceEditRequest {
            path: "notes.md".to_string(),
            anchor: anchor.clone(),
            replacement: "beta replaced".to_string(),
            preview_only: false,
            preview_hash: None,
        },
    )
    .unwrap();
    let missing_preview_hash = execute_tool(
        &package,
        &bundle,
        executor.clone(),
        "workspace_edit",
        "tool.call.edit.missing_preview",
        missing_preview_hash_args,
    );
    assert_eq!(
        missing_preview_hash
            .record
            .effect_result
            .unwrap()
            .terminal_status,
        EffectTerminalStatus::Failed
    );

    fs::write(
        workspace_root.join("notes.md"),
        "alpha one\nchanged\nalpha three\n",
    )
    .unwrap();
    let stale_args = ContentRef::new("content.args.edit.stale");
    args.insert(
        stale_args.clone(),
        &WorkspaceEditRequest {
            path: "notes.md".to_string(),
            anchor,
            replacement: "beta replaced".to_string(),
            preview_only: false,
            preview_hash: Some(preview.preview_hash),
        },
    )
    .unwrap();
    let stale = execute_tool(
        &package,
        &bundle,
        executor,
        "workspace_edit",
        "tool.call.edit.stale",
        stale_args,
    );
    assert_eq!(
        stale.record.effect_result.unwrap().terminal_status,
        EffectTerminalStatus::Failed
    );
}

#[test]
fn write_requires_create_or_overwrite_scope_and_marks_non_reversible_create() {
    let workspace_root = temp_workspace("write");
    let denied_policy = WorkspacePolicy::new(&workspace_root);
    let denied_workspace = Arc::new(BoundedWorkspace::new(denied_policy));
    let args = InMemoryJsonArgumentStore::default();
    let content = InMemoryToolkitContentStore::default();
    let denied_bundle = WorkspaceWriteExecutor::pack_bundle(
        source(),
        permission_policy("policy.fs.write"),
        denied_workspace.policy(),
    )
    .expect("write pack");
    let denied_package = package_for_bundle(&denied_bundle);
    let denied_executor = Arc::new(WorkspaceWriteExecutor::new(
        denied_workspace,
        args.clone(),
        content.clone(),
    ));
    let denied_args = ContentRef::new("content.args.write.denied");
    args.insert(
        denied_args.clone(),
        &WorkspaceWriteRequest {
            path: "created.md".to_string(),
            contents: "new file".to_string(),
            mode: WorkspaceWriteMode::CreateNew,
        },
    )
    .unwrap();
    let denied = execute_tool(
        &denied_package,
        &denied_bundle,
        denied_executor,
        "workspace_write",
        "tool.call.write.denied",
        denied_args,
    );
    assert_eq!(
        denied.record.effect_result.unwrap().terminal_status,
        EffectTerminalStatus::Failed
    );

    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.allow_create = true;
    let allowed_workspace = Arc::new(BoundedWorkspace::new(policy));
    let allowed_bundle = WorkspaceWriteExecutor::pack_bundle(
        source(),
        permission_policy("policy.fs.write"),
        allowed_workspace.policy(),
    )
    .expect("write pack");
    let allowed_package = package_for_bundle(&allowed_bundle);
    let allowed_executor = Arc::new(WorkspaceWriteExecutor::new(
        allowed_workspace,
        args.clone(),
        content.clone(),
    ));
    let create_args = ContentRef::new("content.args.write.create");
    args.insert(
        create_args.clone(),
        &WorkspaceWriteRequest {
            path: "created.md".to_string(),
            contents: "new file".to_string(),
            mode: WorkspaceWriteMode::CreateNew,
        },
    )
    .unwrap();
    let created_outcome = execute_tool(
        &allowed_package,
        &allowed_bundle,
        allowed_executor,
        "workspace_write",
        "tool.call.write.create",
        create_args,
    );
    let created: WorkspaceWriteOutput = content
        .get(&created_outcome.record.result_content_refs[0])
        .expect("write output stored");
    assert!(created.created);
    assert!(created.non_reversible_reason.is_some());
}

#[test]
fn workspace_read_and_search_enforce_size_and_output_bounds() {
    let workspace_root = temp_workspace("bounds");
    fs::write(workspace_root.join("big.txt"), "0123456789").unwrap();
    let mut read_policy = WorkspacePolicy::new(&workspace_root);
    read_policy.max_file_bytes = 8;
    let read_workspace = BoundedWorkspace::new(read_policy);
    let too_large = read_workspace
        .read(&WorkspaceReadRequest {
            path: "big.txt".to_string(),
            max_bytes: None,
        })
        .expect("safe oversized reads return a bounded prefix");
    assert!(too_large.truncated);
    assert_eq!(too_large.content, "01234567");
    assert!(
        too_large
            .warnings
            .iter()
            .any(|warning| warning.contains("workspace_search"))
    );

    fs::write(
        workspace_root.join("long.txt"),
        "needle-abcdefghijklmnopqrstuvwxyz\n",
    )
    .unwrap();
    let mut output_policy = WorkspacePolicy::new(&workspace_root);
    output_policy.max_file_bytes = 1024;
    output_policy.max_output_bytes = 6;
    let output_workspace = BoundedWorkspace::new(output_policy);
    let read = output_workspace
        .read(&WorkspaceReadRequest {
            path: "long.txt".to_string(),
            max_bytes: Some(1000),
        })
        .expect("read clamps caller max_bytes to policy max_output_bytes");
    assert!(read.truncated);
    assert!(read.content.len() <= 6);

    let search = output_workspace
        .search(&WorkspaceSearchRequest {
            pattern: "needle".to_string(),
        })
        .expect("search succeeds");
    assert_eq!(search.matches.len(), 1);
    assert!(search.matches[0].preview.len() <= 6);

    let empty = output_workspace
        .search(&WorkspaceSearchRequest {
            pattern: String::new(),
        })
        .expect_err("empty regex is denied");
    assert!(empty.context().message.contains("must not be empty"));
}

#[cfg(unix)]
#[test]
fn workspace_bounds_deny_symlink_read_and_write_escape() {
    use std::os::unix::fs::symlink;

    let workspace_root = temp_workspace("symlink");
    let outside_root = temp_workspace("outside");
    fs::write(outside_root.join("secret.txt"), "outside secret").unwrap();
    symlink(&outside_root, workspace_root.join("linkdir")).unwrap();

    let read_workspace = BoundedWorkspace::new(WorkspacePolicy::new(&workspace_root));
    let denied = read_workspace
        .read(&WorkspaceReadRequest {
            path: "linkdir/secret.txt".to_string(),
            max_bytes: None,
        })
        .expect_err("intermediate symlink read is denied");
    assert_eq!(denied.kind(), agent_sdk_core::AgentErrorKind::PolicyDenial);

    let mut policy = WorkspacePolicy::new(&workspace_root);
    policy.allow_overwrite = true;
    let workspace = Arc::new(BoundedWorkspace::new(policy));
    let args = InMemoryJsonArgumentStore::default();
    let content = InMemoryToolkitContentStore::default();
    let bundle = WorkspaceWriteExecutor::pack_bundle(
        source(),
        permission_policy("policy.fs.write"),
        workspace.policy(),
    )
    .expect("write pack");
    let package = package_for_bundle(&bundle);
    let executor = Arc::new(WorkspaceWriteExecutor::new(
        workspace,
        args.clone(),
        content,
    ));
    let args_ref = ContentRef::new("content.args.write.symlink");
    args.insert(
        args_ref.clone(),
        &WorkspaceWriteRequest {
            path: "linkdir/secret.txt".to_string(),
            contents: "changed".to_string(),
            mode: WorkspaceWriteMode::Overwrite,
        },
    )
    .unwrap();

    let outcome = execute_tool(
        &package,
        &bundle,
        executor,
        "workspace_write",
        "tool.call.write.symlink",
        args_ref,
    );
    assert_eq!(
        outcome.record.effect_result.unwrap().terminal_status,
        EffectTerminalStatus::Failed
    );
    assert_eq!(
        fs::read_to_string(outside_root.join("secret.txt")).unwrap(),
        "outside secret"
    );
}

#[test]
fn shell_requires_timeout_and_sandbox_policy_and_supports_cancellation() {
    let args = InMemoryJsonArgumentStore::default();
    let content = InMemoryToolkitContentStore::default();
    let bundle = ShellExecutor::pack_bundle(source(), permission_policy("policy.shell"))
        .expect("shell pack");
    let package = package_for_bundle(&bundle);
    let executor = Arc::new(ShellExecutor::new(
        ShellExecutionPolicy::deny_host_execution(),
        args.clone(),
        content,
    ));

    let cancel_ref = ContentRef::new("content.args.shell.cancel");
    args.insert(
        cancel_ref.clone(),
        &ShellRequest {
            argv: vec!["echo".to_string(), "hi".to_string()],
            cwd: None,
            env: Vec::new(),
            timeout_ms: 1000,
            network: false,
            cancel_before_start: true,
        },
    )
    .unwrap();
    let cancelled = execute_tool(
        &package,
        &bundle,
        executor.clone(),
        "shell",
        "tool.call.shell.cancel",
        cancel_ref,
    );
    assert_eq!(
        cancelled.record.effect_result.unwrap().terminal_status,
        EffectTerminalStatus::Cancelled
    );

    let deny_ref = ContentRef::new("content.args.shell.deny");
    args.insert(
        deny_ref.clone(),
        &ShellRequest {
            argv: vec!["echo".to_string(), "hi".to_string()],
            cwd: None,
            env: Vec::new(),
            timeout_ms: 1000,
            network: false,
            cancel_before_start: false,
        },
    )
    .unwrap();
    let denied = execute_tool(
        &package,
        &bundle,
        executor,
        "shell",
        "tool.call.shell.deny",
        deny_ref,
    );
    assert_eq!(
        denied.record.effect_result.unwrap().terminal_status,
        EffectTerminalStatus::Failed
    );
}

#[test]
fn resource_reader_routes_uri_schemes_to_content_refs() {
    let args = InMemoryJsonArgumentStore::default();
    let content = InMemoryToolkitContentStore::default();
    let mut router = agent_sdk_core::ResourceRouter::new();
    router.register(InMemoryResourceResolver::new(
        "memory",
        ContentRef::new("content.memory.resource"),
        source(),
        permission_policy("policy.memory.read"),
    ));
    let bundle =
        ResourceReaderExecutor::pack_bundle(source(), permission_policy("policy.memory.read"))
            .expect("resource pack");
    let package = package_for_bundle(&bundle);
    let executor = Arc::new(ResourceReaderExecutor::new(
        router,
        args.clone(),
        content.clone(),
    ));
    let args_ref = ContentRef::new("content.args.resource");
    args.insert(
        args_ref.clone(),
        &ResourceReaderRequest {
            uri: "memory://summary".to_string(),
            max_bytes: 1024,
        },
    )
    .unwrap();

    let outcome = execute_tool(
        &package,
        &bundle,
        executor,
        "resource_read",
        "tool.call.resource",
        args_ref,
    );
    let resolved: agent_sdk_core::ResourceResolution =
        content.get(&outcome.record.result_content_refs[0]).unwrap();
    assert_eq!(resolved.scheme.as_str(), "memory");
    assert_eq!(
        resolved.content_ref,
        ContentRef::new("content.memory.resource")
    );
}

#[test]
fn tool_discovery_is_read_only_until_package_delta_activation() {
    let workspace_root = temp_workspace("discovery");
    let workspace = WorkspacePolicy::new(&workspace_root);
    let hidden = WorkspaceSearchExecutor::pack_bundle(
        source(),
        permission_policy("policy.fs.read"),
        &workspace,
    )
    .expect("hidden search pack");
    let mut index = ToolDiscoveryIndex::new();
    index.insert(hidden.snapshot.clone());
    assert_eq!(index.search("search").len(), 1);

    let discovery_bundle =
        ToolDiscoveryExecutor::pack_bundle(source(), package_policy("policy.discovery.activate"))
            .expect("discovery pack");
    let base = package_for_bundle(&discovery_bundle);
    assert_eq!(base.capabilities.len(), 1);

    let delta = index
        .activation_delta(
            hidden.snapshot.pack_id.as_str(),
            &base,
            source(),
            package_policy("policy.discovery.activate"),
        )
        .expect("activation delta created");
    let next = base
        .apply_delta(delta)
        .expect("next package activates candidate");
    assert_eq!(base.capabilities.len(), 1);
    assert_eq!(next.capabilities.len(), 2);
    assert_ne!(base.fingerprint().unwrap(), next.fingerprint().unwrap());
}

fn execute_tool(
    package: &RuntimePackage,
    bundle: &ToolkitPackBundle,
    executor: Arc<dyn agent_sdk_core::ToolExecutor>,
    tool_name: &str,
    tool_call_id: &str,
    args_ref: ContentRef,
) -> agent_sdk_core::ToolExecutionOutcome {
    let snapshot =
        ToolRegistrySnapshot::from_runtime_package(package, bundle.routes.clone()).unwrap();
    let mut executors = ToolExecutorRegistry::new();
    executors.register(executor).unwrap();
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy));
    coordinator
        .execute(
            &agent_sdk_core::testing::FakeJournalStore::default(),
            ToolCallRequest {
                tool_call_id: ToolCallId::new(tool_call_id),
                canonical_tool_name: CanonicalToolName::new(tool_name),
                source: SourceRef::with_kind(SourceKind::Sdk, "source.model.tool_call"),
                requested_args_refs: vec![args_ref],
                redacted_args_summary: format!("{tool_name} args redacted"),
                idempotency_key: None,
                dedupe_key: None,
            },
            ToolExecutionContext::new(
                RunId::new("run.toolkit.test"),
                AgentId::new("agent.toolkit.test"),
                source(),
                package.fingerprint().unwrap().as_str(),
            ),
        )
        .expect("tool coordinator returns outcome")
}

fn assert_has_truncation_guidance(output: &WorkspaceReadOutput) {
    assert!(
        output.warnings.iter().any(|warning| {
            warning.contains("workspace_search")
                || warning.contains("grep")
                || warning.contains("narrower")
        }),
        "expected truncation guidance in warnings: {:?}",
        output.warnings
    );
}

fn assert_no_raw_binary_prefix(output: &WorkspaceReadOutput) {
    assert!(
        !output.content.contains('\0'),
        "model-visible content should not contain NUL/raw binary bytes"
    );
    assert!(
        !output.content.starts_with("%PDF-")
            && !output.content.starts_with("PK\u{3}\u{4}")
            && !output.content.starts_with("\u{89}PNG"),
        "model-visible content should be summary/text, not raw file magic"
    );
}

fn package_for_bundle(bundle: &ToolkitPackBundle) -> RuntimePackage {
    bundle
        .install_into(package_builder("package.toolkit.test"))
        .build()
        .expect("package builds")
}

fn package_builder(package_id: &str) -> RuntimePackageBuilder {
    RuntimePackage::builder(RuntimePackageId::new(package_id))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: AgentId::new("agent.toolkit.test"),
            name: "toolkit test".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
}

fn temp_workspace(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("agent-sdk-toolkit-{label}-{nonce}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("notes.md"),
        include_str!("fixtures/workspace/notes.md"),
    )
    .unwrap();
    fs::write(
        root.join("README.txt"),
        include_str!("fixtures/workspace/README.txt"),
    )
    .unwrap();
    root
}

fn install_reader_fixture(root: &std::path::Path, name: &str) {
    let bytes = match name {
        "brief.pdf" => include_bytes!("fixtures/workspace/readers/brief.pdf").as_slice(),
        "pixel.png" => include_bytes!("fixtures/workspace/readers/pixel.png").as_slice(),
        "photo.dng" => include_bytes!("fixtures/workspace/readers/photo.dng").as_slice(),
        "doc.docx" => include_bytes!("fixtures/workspace/readers/doc.docx").as_slice(),
        "bundle.zip" => include_bytes!("fixtures/workspace/readers/bundle.zip").as_slice(),
        "huge.docx" => include_bytes!("fixtures/workspace/readers/huge.docx").as_slice(),
        "sample.tar" => include_bytes!("fixtures/workspace/readers/sample.tar").as_slice(),
        "sample.tgz" => include_bytes!("fixtures/workspace/readers/sample.tgz").as_slice(),
        "sample.txt.gz" => include_bytes!("fixtures/workspace/readers/sample.txt.gz").as_slice(),
        "huge.txt.gz" => include_bytes!("fixtures/workspace/readers/huge.txt.gz").as_slice(),
        "huge.bin.gz" => include_bytes!("fixtures/workspace/readers/huge.bin.gz").as_slice(),
        "huge.tgz" => include_bytes!("fixtures/workspace/readers/huge.tgz").as_slice(),
        "sample.sqlite" => include_bytes!("fixtures/workspace/readers/sample.sqlite").as_slice(),
        "malformed.sqlite" => {
            include_bytes!("fixtures/workspace/readers/malformed.sqlite").as_slice()
        }
        "ocr.png" => include_bytes!("fixtures/workspace/readers/ocr.png").as_slice(),
        "ocr.png.ocr.txt" => {
            include_bytes!("fixtures/workspace/readers/ocr.png.ocr.txt").as_slice()
        }
        "scanned.pdf" => include_bytes!("fixtures/workspace/readers/scanned.pdf").as_slice(),
        "scanned.pdf.ocr.txt" => {
            include_bytes!("fixtures/workspace/readers/scanned.pdf.ocr.txt").as_slice()
        }
        "photo-preview.dng" => {
            include_bytes!("fixtures/workspace/readers/photo-preview.dng").as_slice()
        }
        "photo-preview.dng.aae" => {
            include_bytes!("fixtures/workspace/readers/photo-preview.dng.aae").as_slice()
        }
        "legacy.doc" => include_bytes!("fixtures/workspace/readers/legacy.doc").as_slice(),
        "legacy.doc.txt" => include_bytes!("fixtures/workspace/readers/legacy.doc.txt").as_slice(),
        "legacy.xls" => include_bytes!("fixtures/workspace/readers/legacy.xls").as_slice(),
        "legacy.xls.txt" => include_bytes!("fixtures/workspace/readers/legacy.xls.txt").as_slice(),
        "legacy.ppt" => include_bytes!("fixtures/workspace/readers/legacy.ppt").as_slice(),
        "legacy.ppt.txt" => include_bytes!("fixtures/workspace/readers/legacy.ppt.txt").as_slice(),
        "corrupt.png" => include_bytes!("fixtures/workspace/readers/corrupt.png").as_slice(),
        "corrupt.pdf" => include_bytes!("fixtures/workspace/readers/corrupt.pdf").as_slice(),
        "encrypted-placeholder.pdf" => {
            include_bytes!("fixtures/workspace/readers/encrypted-placeholder.pdf").as_slice()
        }
        "corrupt.zip" => include_bytes!("fixtures/workspace/readers/corrupt.zip").as_slice(),
        "corrupt.tar" => include_bytes!("fixtures/workspace/readers/corrupt.tar").as_slice(),
        _ => panic!("unknown reader fixture {name}"),
    };
    fs::write(root.join(name), bytes).unwrap();
}

fn source() -> SourceRef {
    SourceRef::with_kind(SourceKind::Sdk, "source.sdk.toolkit")
}

fn permission_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Permission, id)
}

fn package_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}
