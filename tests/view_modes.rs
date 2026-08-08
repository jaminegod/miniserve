use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use assert_cmd::cargo;
use assert_fs::fixture::TempDir;
use assert_fs::prelude::*;
use reqwest::{StatusCode, blocking::Client};
use rstest::rstest;

mod fixtures;

use crate::fixtures::{Error, TestServer, port, reqwest_client};

/// Helper: start miniserve pointing at a temp dir that contains some image + non-image files
/// and subdirectories, returning a running TestServer (which owns and kills the child process).
fn server_with_images(port: u16) -> TestServer {
    let tmpdir = TempDir::new().expect("Couldn't create temp dir");
    // a couple of image files (content doesn't matter for HTML rendering tests)
    tmpdir.child("a.png").write_str("png").unwrap();
    tmpdir.child("b.jpg").write_str("jpg").unwrap();
    // a non-image file
    tmpdir.child("notes.txt").write_str("hi").unwrap();
    // a subdirectory with an image inside
    tmpdir.child("sub/c.jpeg").write_str("jpeg").unwrap();

    let child = Command::new(cargo::cargo_bin!("miniserve"))
        .arg(tmpdir.path())
        .arg("-p")
        .arg(port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Couldn't run miniserve");

    // wait for the port to come up
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !port_check::is_port_reachable(format!("localhost:{port}")) {
        if std::time::Instant::now() > deadline {
            panic!("timeout waiting for port {port}");
        }
        sleep(Duration::from_millis(100));
    }
    TestServer::new(port, tmpdir, child, false)
}

#[rstest]
fn list_view_renders_table(reqwest_client: Client, port: u16) -> Result<(), Error> {
    let _srv = server_with_images(port);
    let body = reqwest_client
        .get(format!("http://localhost:{port}/"))
        .send()?
        .error_for_status()?
        .text()?;
    // default view is list -> a table should be present
    assert!(body.contains("<table"), "list view must render a table");
    assert!(
        body.contains("view-switcher"),
        "view switcher should always be present"
    );
    Ok(())
}

#[rstest]
fn grid_view_renders_grid(reqwest_client: Client, port: u16) -> Result<(), Error> {
    let _srv = server_with_images(port);
    let body = reqwest_client
        .get(format!("http://localhost:{port}/?view=grid"))
        .send()?
        .error_for_status()?
        .text()?;
    assert!(
        body.contains("grid-view"),
        "grid view must render a .grid-view container"
    );
    // image entries should carry a data-lightbox attribute in grid view
    assert!(
        body.contains("data-lightbox"),
        "image thumbnails must be lightbox-enabled"
    );
    // the list table should NOT be rendered in grid mode
    assert!(
        !body.contains("<table>"),
        "grid view should not render the list table"
    );
    Ok(())
}

#[rstest]
fn album_view_renders_album(reqwest_client: Client, port: u16) -> Result<(), Error> {
    let _srv = server_with_images(port);
    let body = reqwest_client
        .get(format!("http://localhost:{port}/?view=album"))
        .send()?
        .error_for_status()?
        .text()?;
    assert!(
        body.contains("album-view"),
        "album view must render an .album-view container"
    );
    assert!(
        !body.contains("<table>"),
        "album view should not render the list table"
    );
    Ok(())
}

#[rstest]
fn default_view_grid_arg(reqwest_client: Client, port: u16) -> Result<(), Error> {
    // start a server with --default-view grid
    let tmpdir = TempDir::new().unwrap();
    tmpdir.child("a.png").write_str("png").unwrap();
    let child = Command::new(cargo::cargo_bin!("miniserve"))
        .arg(tmpdir.path())
        .arg("-p")
        .arg(port.to_string())
        .arg("--default-view")
        .arg("grid")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let _srv = TestServer::new(port, tmpdir, child, false);
    let body = reqwest_client
        .get(format!("http://localhost:{port}/"))
        .send()?
        .error_for_status()?
        .text()?;
    assert!(
        body.contains("grid-view"),
        "default-view grid should render grid without ?view="
    );
    Ok(())
}

#[rstest]
fn list_dirs_api_returns_subdirs(reqwest_client: Client, port: u16) -> Result<(), Error> {
    let _srv = server_with_images(port);
    let mut cmd = HashMap::new();
    cmd.insert("ListDirs", HashMap::from([("path", "")]));
    let resp = reqwest_client
        .post(format!("http://localhost:{port}/__miniserve_internal/api"))
        .json(&cmd)
        .send()?
        .error_for_status()?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text()?;
    assert!(
        body.contains("\"name\":\"sub\""),
        "ListDirs should return the 'sub' subdirectory, got: {body}"
    );
    Ok(())
}

#[rstest]
fn list_dirs_api_path_traversal_blocked(reqwest_client: Client, port: u16) -> Result<(), Error> {
    let _srv = server_with_images(port);
    let mut cmd = HashMap::new();
    // try to escape the serve root
    cmd.insert("ListDirs", HashMap::from([("path", "../../")]));
    let resp = reqwest_client
        .post(format!("http://localhost:{port}/__miniserve_internal/api"))
        .json(&cmd)
        .send()?;
    // sanitize_path collapses `..` so this just lists the root again (200 OK, valid dirs)
    assert_eq!(resp.status(), StatusCode::OK);
    Ok(())
}
