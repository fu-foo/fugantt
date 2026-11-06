//! Making, renaming, ordering and deleting projects, through the API and the
//! screens.

mod common;

use common::Server;

const OPEN: (&str, &str) = ("FUGANTT_NO_AUTH", "yes-everyone-on-this-network-can-edit");
const JSON: &str = "Content-Type: application/json";
const FORM: &str = "Content-Type: application/x-www-form-urlencoded";

fn status(response: &str) -> u16 {
    response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("ステータスが読めない")
}

/// The project names, in the order the list gives them.
fn names(server: &Server) -> Vec<String> {
    let (code, body) = server.body("/api/projects");
    assert_eq!(code, 200);

    body.split("\"name\":\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap().to_owned())
        .collect()
}

fn create(server: &Server, name: &str) -> String {
    server.send(
        "POST",
        "/api/projects",
        &[JSON],
        &format!("{{\"name\":\"{name}\"}}"),
    )
}

#[test]
fn projects_can_be_made_renamed_ordered_and_deleted() {
    let server = Server::start("projects", &[OPEN]);

    for name in ["alpha", "bravo", "charlie"] {
        let made = create(&server, name);
        assert_eq!(status(&made), 200, "{made}");
        assert!(made.contains(&format!("\"id\":\"{name}\"")), "{made}");
    }

    // A new plan goes to the end, not wherever its name would sort.
    assert_eq!(
        create(&server, "aaa").split_whitespace().nth(1),
        Some("200")
    );
    assert_eq!(names(&server), ["alpha", "bravo", "charlie", "aaa"]);

    // A name that is taken is refused, whatever its case.
    assert_eq!(status(&create(&server, "Bravo")), 400);

    // Renaming keeps the id, so the URL keeps working.
    let renamed = server.send(
        "POST",
        "/api/projects/alpha/name",
        &[JSON],
        r#"{"name":"zulu"}"#,
    );
    assert_eq!(status(&renamed), 200, "{renamed}");
    assert!(renamed.contains("\"id\":\"alpha\""), "{renamed}");
    assert!(renamed.contains("\"name\":\"zulu\""), "{renamed}");
    assert_eq!(server.body("/projects/alpha").0, 200);

    let clash = server.send(
        "POST",
        "/api/projects/bravo/name",
        &[JSON],
        r#"{"name":"ZULU"}"#,
    );
    assert_eq!(status(&clash), 400);

    // The ones named go first; the rest keep their order after them.
    let ordered = server.send(
        "POST",
        "/api/projects/order",
        &[JSON],
        r#"{"ids":["charlie","nothing-here"]}"#,
    );
    assert_eq!(status(&ordered), 200, "{ordered}");
    assert_eq!(names(&server), ["charlie", "zulu", "bravo", "aaa"]);

    // And from the list on the screen, one place at a time.
    let moved = server.send("POST", "/projects/move", &[FORM], "id=aaa&direction=up");
    assert_eq!(status(&moved), 303, "{moved}");
    assert_eq!(names(&server), ["charlie", "zulu", "aaa", "bravo"]);

    let gone = server.send("DELETE", "/api/projects/bravo", &[], "");
    assert_eq!(status(&gone), 200, "{gone}");
    assert_eq!(names(&server), ["charlie", "zulu", "aaa"]);
    assert_ne!(server.body("/projects/bravo").0, 200);
    assert_eq!(
        status(&server.send("DELETE", "/api/projects/bravo", &[], "")),
        404
    );

    // The screens do the same.
    let renamed = server.send("POST", "/projects/aaa/rename", &[FORM], "name=yankee");
    assert_eq!(status(&renamed), 303, "{renamed}");
    let removed = server.send("POST", "/projects/aaa/remove", &[FORM], "");
    assert_eq!(status(&removed), 303, "{removed}");
    assert_eq!(names(&server), ["charlie", "zulu"]);
}

#[test]
fn a_token_for_one_project_cannot_make_or_delete_projects() {
    let server = Server::start("projects-token", &[OPEN]);
    assert_eq!(status(&create(&server, "alpha")), 200);
    assert_eq!(status(&create(&server, "bravo")), 200);

    let issued = server.send(
        "POST",
        "/projects/alpha/tokens",
        &[FORM],
        "name=bot&role=editor",
    );
    let token = issued
        .split("issued=")
        .nth(1)
        .and_then(|rest| rest.split(['#', '&', '\r']).next())
        .expect("トークンが返らない")
        .to_owned();
    let bearer = format!("Authorization: Bearer {token}");

    let send = |method: &str, path: &str, body: &str| {
        status(&server.send(method, path, &[JSON, &bearer], body))
    };

    // Its own plan it may rename.
    assert_eq!(
        send("POST", "/api/projects/alpha/name", r#"{"name":"alpha2"}"#),
        200
    );
    assert_eq!(
        send("POST", "/api/projects/bravo/name", r#"{"name":"x"}"#),
        403
    );

    // Anything about the list itself is for a token that reaches all of it.
    assert_eq!(send("POST", "/api/projects", r#"{"name":"charlie"}"#), 403);
    assert_eq!(
        send("POST", "/api/projects/order", r#"{"ids":["bravo"]}"#),
        403
    );
    assert_eq!(send("DELETE", "/api/projects/alpha", ""), 403);

    assert_eq!(names(&server), ["alpha2", "bravo"]);
}

#[test]
fn a_token_for_every_project_can_make_order_and_delete_them() {
    let server = Server::start("projects-wide", &[OPEN]);
    assert_eq!(status(&create(&server, "alpha")), 200);

    let issued = server.send("POST", "/admin/tokens", &[FORM], "name=bot&role=editor");
    let token = issued
        .split("issued=")
        .nth(1)
        .and_then(|rest| rest.split(['#', '&', '\r']).next())
        .expect("トークンが返らない")
        .to_owned();
    let bearer = format!("Authorization: Bearer {token}");

    let send = |method: &str, path: &str, body: &str| {
        status(&server.send(method, path, &[JSON, &bearer], body))
    };

    assert_eq!(send("POST", "/api/projects", r#"{"name":"bravo"}"#), 200);
    assert_eq!(
        send("POST", "/api/projects/order", r#"{"ids":["bravo"]}"#),
        200
    );
    assert_eq!(names(&server), ["bravo", "alpha"]);
    assert_eq!(send("DELETE", "/api/projects/alpha", ""), 200);
    assert_eq!(names(&server), ["bravo"]);

    // A token that only reads cannot.
    let issued = server.send("POST", "/admin/tokens", &[FORM], "name=reader&role=viewer");
    let token = issued
        .split("issued=")
        .nth(1)
        .and_then(|rest| rest.split(['#', '&', '\r']).next())
        .unwrap()
        .to_owned();
    let reader = format!("Authorization: Bearer {token}");
    let made = server.send("POST", "/api/projects", &[JSON, &reader], r#"{"name":"c"}"#);
    assert_eq!(status(&made), 403);
}
