use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use prost::Message as _;
use tungstenite::client::IntoClientRequest;
use tungstenite::http::HeaderValue;
use tungstenite::protocol::WebSocket;

use super::Mux;
use crate::ticket::Ticket;

/// Rows the banner pane takes, caption included.
const BANNER_ROWS: i32 = 2;

/// The WebSocket subprotocol iTerm2 requires on the handshake.
const SUBPROTOCOL: &str = "api.iterm2.com";

/// Returns the current session when Amaca runs inside iTerm2 with the Python
/// API enabled, which is what creates the control socket.
pub fn detect() -> Option<Mux> {
    let session = session_id()?;
    if !socket_path().exists() {
        eprintln!(
            "warning: iTerm2 found but its API is off: enable Settings > General > Magic > Enable Python API"
        );
        return None;
    }
    Some(Mux::Iterm2(session))
}

/// Session id from `ITERM_SESSION_ID`, which is shaped `w0t0p0:<uuid>`.
fn session_id() -> Option<String> {
    std::env::var("ITERM_SESSION_ID")
        .ok()?
        .rsplit_once(':')
        .map(|(_, id)| id.to_owned())
}

fn socket_path() -> PathBuf {
    let suite = std::env::var("IT2_SUITE").unwrap_or_else(|_| "iTerm2".into());
    let home = std::env::var("HOME").unwrap_or_default();
    Path::new(&home)
        .join("Library/Application Support")
        .join(suite)
        .join("private/socket")
}

/// Splits the current pane horizontally with a 2-row pane above it, running
/// the hidden `ꙮ` command on the ticket, then returns the focus to this pane.
pub fn layout(ticket: &Ticket, cwd: &Path, session: &str) -> Result<()> {
    let exe = std::env::current_exe().context("cannot resolve the amaca executable")?;
    let command = command_line(&exe, &["ꙮ", &ticket.id, &ticket.title, &ticket.summary]);
    let mut connection = Connection::open()?;

    let pane = split_pane(&mut connection, session, &command, cwd)?;
    if let Err(err) = shrink_banner(&mut connection, &pane) {
        eprintln!("warning: cannot resize the iTerm2 banner pane: {err:#}");
    }

    let status = activate_status(connection.request(api::request::activate(session))?)?;
    if status != 0 {
        bail!(
            "iTerm2 cannot focus the session: {}",
            activate_status_name(status)
        );
    }
    Ok(())
}

/// The command line the new pane runs, quoted for the shell-like parser and
/// escaped for the interpolated string iTerm2 evaluates the profile command as.
fn command_line(exe: &Path, args: &[&str]) -> String {
    let mut line = shell_quote(&exe.to_string_lossy());
    for arg in args {
        line.push(' ');
        line.push_str(&shell_quote(arg));
    }
    escape_interpolation(&line)
}

fn shell_quote(text: &str) -> String {
    let mut out = String::from("'");
    for c in text.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// Doubles the backslashes and escapes the double quotes, the only sequences
/// iTerm2 expands while evaluating a strict interpolated string.
fn escape_interpolation(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c == '\\' || c == '"' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn split_pane(conn: &mut Connection, session: &str, command: &str, cwd: &Path) -> Result<String> {
    let properties = [
        ("Custom Command", "Yes".to_owned()),
        ("Command", command.to_owned()),
        ("Custom Directory", "Yes".to_owned()),
        ("Working Directory", cwd.to_string_lossy().into_owned()),
    ]
    .map(|(key, value)| api::ProfileProperty {
        key: Some(key.to_owned()),
        json_value: Some(serde_json::to_string(&value).expect("a string always serializes")),
    });

    let response = conn.request(api::request::split_pane(session, &properties))?;
    match response.submessage {
        Some(api::server::Submessage::SplitPaneResponse(response)) => {
            let status = response.status.unwrap_or_default();
            if status != 0 {
                bail!(
                    "iTerm2 cannot split the pane: {}",
                    split_status_name(status)
                );
            }
            response
                .session_id
                .into_iter()
                .next()
                .context("iTerm2 did not return the new pane")
        }
        Some(api::server::Submessage::Error(error)) => bail!("iTerm2: {error}"),
        _ => bail!("unexpected response to the iTerm2 split request"),
    }
}

/// Gives the fresh pane two rows and hands the rest back to the pane it was
/// split from.
fn shrink_banner(conn: &mut Connection, pane: &str) -> Result<()> {
    let response = conn.request(api::request::list_sessions())?;
    let sessions = match response.submessage {
        Some(api::server::Submessage::ListSessionsResponse(response)) => response,
        Some(api::server::Submessage::Error(error)) => bail!("iTerm2: {error}"),
        _ => bail!("unexpected response to the iTerm2 list request"),
    };

    let tab = sessions
        .windows
        .iter()
        .flat_map(|window| &window.tabs)
        .find(|tab| tab.root.as_ref().is_some_and(|root| contains(root, pane)))
        .context("iTerm2 did not list the new pane")?;
    let mut root = tab.root.clone().expect("checked by contains");

    if !shrink(&mut root, pane) {
        bail!("iTerm2 listed the new pane without its split");
    }

    let status =
        layout_status(conn.request(api::request::set_tab_layout(tab.tab_id.as_deref(), &root))?)?;
    if status != 0 {
        bail!("iTerm2 rejects the layout: {}", layout_status_name(status));
    }
    Ok(())
}

fn contains(node: &api::SplitTreeNode, pane: &str) -> bool {
    node.links.iter().any(|link| match &link.child {
        Some(api::split_tree_link::Child::Session(session)) => {
            session.unique_identifier.as_deref() == Some(pane)
        }
        Some(api::split_tree_link::Child::Node(node)) => contains(node, pane),
        None => false,
    })
}

/// Finds the split holding the new pane, sets it to two rows and grows the
/// other side by the rows it gave up, keeping every container the sum of its
/// children.
fn shrink(node: &mut api::SplitTreeNode, pane: &str) -> bool {
    let index = node.links.iter().position(|link| match &link.child {
        Some(api::split_tree_link::Child::Session(session)) => {
            session.unique_identifier.as_deref() == Some(pane)
        }
        _ => false,
    });

    if let Some(index) = index {
        let old = match &node.links[index].child {
            Some(api::split_tree_link::Child::Session(session)) => {
                session.grid_size.as_ref().and_then(|size| size.height)
            }
            _ => None,
        };
        let Some(old) = old else { return true };
        let delta = old - BANNER_ROWS;
        if delta <= 0 {
            return true;
        }

        if let Some(api::split_tree_link::Child::Session(session)) = &mut node.links[index].child
            && let Some(size) = &mut session.grid_size
        {
            size.height = Some(BANNER_ROWS);
        }
        if let Some(other) = node.links.get_mut(1 - index) {
            grow(other, delta);
        }
        return true;
    }

    node.links.iter_mut().any(|link| match &mut link.child {
        Some(api::split_tree_link::Child::Node(node)) => shrink(node, pane),
        _ => false,
    })
}

/// Grows a subtree by `delta` rows. Every child of a side-by-side split shares
/// the container height; a stacked split gives the rows to the child nearest
/// the new pane.
fn grow(link: &mut api::SplitTreeLink, delta: i32) {
    match &mut link.child {
        Some(api::split_tree_link::Child::Session(session)) => {
            if let Some(size) = &mut session.grid_size {
                size.height = size.height.map(|height| height + delta);
            }
        }
        Some(api::split_tree_link::Child::Node(node)) => {
            if node.vertical.unwrap_or_default() {
                for link in &mut node.links {
                    grow(link, delta);
                }
            } else if let Some(first) = node.links.first_mut() {
                grow(first, delta);
            }
        }
        None => {}
    }
}

fn activate_status(response: api::ServerOriginatedMessage) -> Result<i32> {
    match response.submessage {
        Some(api::server::Submessage::ActivateResponse(response)) => {
            Ok(response.status.unwrap_or_default())
        }
        Some(api::server::Submessage::Error(error)) => bail!("iTerm2: {error}"),
        _ => bail!("unexpected response to the iTerm2 activate request"),
    }
}

fn layout_status(response: api::ServerOriginatedMessage) -> Result<i32> {
    match response.submessage {
        Some(api::server::Submessage::SetTabLayoutResponse(response)) => {
            Ok(response.status.unwrap_or_default())
        }
        Some(api::server::Submessage::Error(error)) => bail!("iTerm2: {error}"),
        _ => bail!("unexpected response to the iTerm2 layout request"),
    }
}

fn split_status_name(status: i32) -> String {
    match status {
        0 => "OK".into(),
        1 => "session not found".into(),
        2 => "invalid profile name".into(),
        3 => "cannot split".into(),
        4 => "malformed profile property".into(),
        _ => format!("status {status}"),
    }
}

fn activate_status_name(status: i32) -> String {
    match status {
        0 => "OK".into(),
        1 => "bad identifier".into(),
        2 => "invalid option".into(),
        _ => format!("status {status}"),
    }
}

fn layout_status_name(status: i32) -> String {
    match status {
        0 => "OK".into(),
        1 => "bad tab id".into(),
        2 => "wrong tree".into(),
        3 => "invalid size".into(),
        _ => format!("status {status}"),
    }
}

/// A connection to the iTerm2 control socket.
struct Connection {
    socket: WebSocket<UnixStream>,
    next_id: i64,
}

impl Connection {
    fn open() -> Result<Self> {
        let path = socket_path();
        let stream = UnixStream::connect(&path).with_context(|| {
            format!(
                "cannot connect to iTerm2 at {}: enable Settings > General > Magic > \
                 Enable Python API",
                path.display()
            )
        })?;
        // A stuck iTerm2 must not hang Amaca forever.
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;

        let (cookie, key) = credentials()?;
        let mut request = "ws://localhost/"
            .into_client_request()
            .context("cannot build the iTerm2 handshake")?;
        let headers = request.headers_mut();
        headers.insert("origin", HeaderValue::from_static("ws://localhost/"));
        // iTerm2 parses this as "<library> <version>" and requires a known
        // library at or above its minimum version.
        headers.insert(
            "x-iterm2-library-version",
            HeaderValue::from_static("python 1.0"),
        );
        headers.insert("x-iterm2-disable-auth-ui", HeaderValue::from_static("true"));
        headers.insert("x-iterm2-advisory-name", HeaderValue::from_static("amaca"));
        headers.insert(
            "sec-websocket-protocol",
            HeaderValue::from_static(SUBPROTOCOL),
        );
        if let Some(cookie) = cookie {
            headers.insert("x-iterm2-cookie", HeaderValue::from_str(&cookie)?);
        }
        if let Some(key) = key {
            headers.insert("x-iterm2-key", HeaderValue::from_str(&key)?);
        }

        let (socket, _response) = tungstenite::client::client(request, stream).context(
            "iTerm2 rejects the connection: enable Settings > General > Magic > \
                      Enable Python API",
        )?;
        Ok(Self { socket, next_id: 0 })
    }

    fn request(
        &mut self,
        message: api::ClientOriginatedMessage,
    ) -> Result<api::ServerOriginatedMessage> {
        let id = self.next_id;
        self.next_id += 1;
        let mut message = message;
        message.id = Some(id);
        self.socket
            .send(tungstenite::Message::Binary(message.encode_to_vec().into()))
            .context("cannot send to iTerm2")?;

        loop {
            match self.socket.read().context("cannot read from iTerm2")? {
                tungstenite::Message::Binary(bytes) => {
                    let response = api::ServerOriginatedMessage::decode(bytes)?;
                    if response.id == Some(id) {
                        return Ok(response);
                    }
                }
                tungstenite::Message::Close(_) => bail!("iTerm2 closed the connection"),
                _ => {}
            }
        }
    }
}

/// Cookie and key for the handshake. They come from the environment when
/// iTerm2 exported them, otherwise from a direct Apple Event to iTerm2, which
/// asks the user for permission the first time.
fn credentials() -> Result<(Option<String>, Option<String>)> {
    let cookie = std::env::var("ITERM2_COOKIE").ok();
    let key = std::env::var("ITERM2_KEY").ok();
    if cookie.is_some() {
        return Ok((cookie, key));
    }
    let reply = request_cookie()?;
    let (cookie, key) = reply
        .split_once(' ')
        .context("iTerm2 returned a malformed cookie")?;
    Ok((Some(cookie.to_owned()), Some(key.to_owned())))
}

fn request_cookie() -> Result<String> {
    use anyhow::anyhow;
    use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventSendOptions, NSString};

    // FourCC codes from iTerm2.sdef: event class "Itrm", event "rqck",
    // parameter "and key for app named" "Rcsn".
    const EVENT_CLASS: u32 = u32::from_be_bytes(*b"Itrm");
    const EVENT_ID: u32 = u32::from_be_bytes(*b"rqck");
    const APP_NAMED: u32 = u32::from_be_bytes(*b"Rcsn");

    let target = NSAppleEventDescriptor::descriptorWithBundleIdentifier(&NSString::from_str(
        "com.googlecode.iterm2",
    ));
    // kAutoGenerateReturnID, kAnyTransactionID.
    let event = NSAppleEventDescriptor::appleEventWithEventClass_eventID_targetDescriptor_returnID_transactionID(
        EVENT_CLASS,
        EVENT_ID,
        Some(&target),
        -1,
        0,
    );
    let name = NSAppleEventDescriptor::descriptorWithString(&NSString::from_str("amaca"));
    event.setParamDescriptor_forKeyword(&name, APP_NAMED);

    let reply = event
        .sendEventWithOptions_timeout_error(
            NSAppleEventSendOptions::WaitForReply | NSAppleEventSendOptions::CanInteract,
            60.0,
        )
        .map_err(|error| {
            let message = format!(
                "cannot request the iTerm2 cookie: {} ({})",
                error.localizedDescription(),
                error.code()
            );
            match error.code() {
                // errAEEventNotPermitted: the Automation permission was refused.
                -1743 => anyhow!(
                    "{message}: allow amaca to control iTerm2 in System Settings > \
                     Privacy & Security > Automation"
                ),
                _ => anyhow!("{message}"),
            }
        })?;
    // iTerm2 answers with an event reply whose first item is the "cookie key"
    // string, not with a bare string.
    let reply = reply_string(&reply).context(
        "iTerm2 did not return a cookie: enable Settings > General > Magic > Enable Python API",
    )?;
    Ok(reply)
}

/// Opens an Apple Event reply until it reaches the string inside it: iTerm2
/// nests the "cookie key" value one or more descriptors deep.
fn reply_string(reply: &objc2_foundation::NSAppleEventDescriptor) -> Option<String> {
    if let Some(string) = reply.stringValue() {
        return Some(string.to_string());
    }
    // descriptorAtIndex is 1-based; index 0 is always nil.
    let count = reply.numberOfItems();
    for index in 1..=count {
        let Some(child) = reply.descriptorAtIndex(index) else {
            continue;
        };
        if let Some(string) = reply_string(&child) {
            return Some(string);
        }
    }
    None
}

/// The few messages of iTerm2's protobuf protocol Amaca needs.
mod api {
    use prost::Message;

    #[derive(Clone, PartialEq, Message)]
    pub struct ClientOriginatedMessage {
        #[prost(int64, optional, tag = "1")]
        pub id: Option<i64>,
        #[prost(oneof = "client::Submessage", tags = "106, 109, 114, 122")]
        pub submessage: Option<client::Submessage>,
    }

    pub mod client {
        // The variant names mirror the protobuf fields they carry.
        #[allow(clippy::enum_variant_names)]
        #[derive(Clone, PartialEq, prost::Oneof)]
        pub enum Submessage {
            #[prost(message, tag = "106")]
            ListSessionsRequest(super::ListSessionsRequest),
            #[prost(message, tag = "109")]
            SplitPaneRequest(super::SplitPaneRequest),
            #[prost(message, tag = "114")]
            ActivateRequest(super::ActivateRequest),
            #[prost(message, tag = "122")]
            SetTabLayoutRequest(super::SetTabLayoutRequest),
        }
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct ServerOriginatedMessage {
        #[prost(int64, optional, tag = "1")]
        pub id: Option<i64>,
        #[prost(oneof = "server::Submessage", tags = "2, 106, 109, 114, 122")]
        pub submessage: Option<server::Submessage>,
    }

    pub mod server {
        #[derive(Clone, PartialEq, prost::Oneof)]
        pub enum Submessage {
            #[prost(string, tag = "2")]
            Error(String),
            #[prost(message, tag = "106")]
            ListSessionsResponse(super::ListSessionsResponse),
            #[prost(message, tag = "109")]
            SplitPaneResponse(super::SplitPaneResponse),
            #[prost(message, tag = "114")]
            ActivateResponse(super::ActivateResponse),
            #[prost(message, tag = "122")]
            SetTabLayoutResponse(super::SetTabLayoutResponse),
        }
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct ProfileProperty {
        #[prost(string, optional, tag = "1")]
        pub key: Option<String>,
        #[prost(string, optional, tag = "2")]
        pub json_value: Option<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct SplitPaneRequest {
        #[prost(string, optional, tag = "1")]
        pub session: Option<String>,
        #[prost(int32, optional, tag = "2")]
        pub split_direction: Option<i32>,
        #[prost(bool, optional, tag = "3")]
        pub before: Option<bool>,
        #[prost(message, repeated, tag = "5")]
        pub custom_profile_properties: Vec<ProfileProperty>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct SplitPaneResponse {
        #[prost(int32, optional, tag = "1")]
        pub status: Option<i32>,
        #[prost(string, repeated, tag = "2")]
        pub session_id: Vec<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct ListSessionsRequest {}

    #[derive(Clone, PartialEq, Message)]
    pub struct ListSessionsResponse {
        #[prost(message, repeated, tag = "1")]
        pub windows: Vec<Window>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct Window {
        #[prost(message, repeated, tag = "1")]
        pub tabs: Vec<Tab>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct Tab {
        #[prost(message, optional, tag = "3")]
        pub root: Option<SplitTreeNode>,
        #[prost(string, optional, tag = "2")]
        pub tab_id: Option<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct SplitTreeNode {
        #[prost(bool, optional, tag = "1")]
        pub vertical: Option<bool>,
        #[prost(message, repeated, tag = "2")]
        pub links: Vec<SplitTreeLink>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct SplitTreeLink {
        #[prost(oneof = "split_tree_link::Child", tags = "1, 2")]
        pub child: Option<split_tree_link::Child>,
    }

    pub mod split_tree_link {
        #[derive(Clone, PartialEq, prost::Oneof)]
        pub enum Child {
            #[prost(message, tag = "1")]
            Session(super::SessionSummary),
            #[prost(message, tag = "2")]
            Node(super::SplitTreeNode),
        }
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct SessionSummary {
        #[prost(string, optional, tag = "1")]
        pub unique_identifier: Option<String>,
        #[prost(message, optional, tag = "2")]
        pub frame: Option<Frame>,
        #[prost(message, optional, tag = "3")]
        pub grid_size: Option<Size>,
        #[prost(string, optional, tag = "4")]
        pub title: Option<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct Frame {
        #[prost(message, optional, tag = "1")]
        pub origin: Option<Point>,
        #[prost(message, optional, tag = "2")]
        pub size: Option<Size>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct Point {
        #[prost(int32, optional, tag = "1")]
        pub x: Option<i32>,
        #[prost(int32, optional, tag = "2")]
        pub y: Option<i32>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct Size {
        #[prost(int32, optional, tag = "1")]
        pub width: Option<i32>,
        #[prost(int32, optional, tag = "2")]
        pub height: Option<i32>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct ActivateRequest {
        #[prost(oneof = "activate_request::Identifier", tags = "1, 2, 3")]
        pub identifier: Option<activate_request::Identifier>,
        #[prost(bool, optional, tag = "5")]
        pub select_tab: Option<bool>,
        #[prost(bool, optional, tag = "6")]
        pub select_session: Option<bool>,
    }

    pub mod activate_request {
        // The variant names mirror the protobuf fields they carry.
        #[allow(clippy::enum_variant_names)]
        #[derive(Clone, PartialEq, prost::Oneof)]
        pub enum Identifier {
            #[prost(string, tag = "1")]
            WindowId(String),
            #[prost(string, tag = "2")]
            TabId(String),
            #[prost(string, tag = "3")]
            SessionId(String),
        }
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct ActivateResponse {
        #[prost(int32, optional, tag = "1")]
        pub status: Option<i32>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct SetTabLayoutRequest {
        #[prost(message, optional, tag = "1")]
        pub root: Option<SplitTreeNode>,
        #[prost(string, optional, tag = "2")]
        pub tab_id: Option<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    pub struct SetTabLayoutResponse {
        #[prost(int32, optional, tag = "1")]
        pub status: Option<i32>,
    }

    /// Request builders.
    pub mod request {
        use super::{
            ActivateRequest, ClientOriginatedMessage, ListSessionsRequest, ProfileProperty,
            SetTabLayoutRequest, SplitPaneRequest, SplitTreeNode, activate_request,
            client::Submessage,
        };

        pub fn split_pane(
            session: &str,
            properties: &[ProfileProperty],
        ) -> ClientOriginatedMessage {
            ClientOriginatedMessage {
                id: None,
                submessage: Some(Submessage::SplitPaneRequest(SplitPaneRequest {
                    session: Some(session.to_owned()),
                    // HORIZONTAL: the divider is horizontal, the panes stack.
                    split_direction: Some(1),
                    before: Some(true),
                    custom_profile_properties: properties.to_vec(),
                })),
            }
        }

        pub fn list_sessions() -> ClientOriginatedMessage {
            ClientOriginatedMessage {
                id: None,
                submessage: Some(Submessage::ListSessionsRequest(ListSessionsRequest {})),
            }
        }

        pub fn set_tab_layout(
            tab_id: Option<&str>,
            root: &SplitTreeNode,
        ) -> ClientOriginatedMessage {
            ClientOriginatedMessage {
                id: None,
                submessage: Some(Submessage::SetTabLayoutRequest(SetTabLayoutRequest {
                    root: Some(root.clone()),
                    tab_id: tab_id.map(str::to_owned),
                })),
            }
        }

        pub fn activate(session: &str) -> ClientOriginatedMessage {
            ClientOriginatedMessage {
                id: None,
                submessage: Some(Submessage::ActivateRequest(ActivateRequest {
                    identifier: Some(activate_request::Identifier::SessionId(session.to_owned())),
                    select_tab: Some(true),
                    select_session: Some(true),
                })),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{api, command_line, escape_interpolation, shell_quote, shrink};
    use std::path::Path;

    fn leaf(id: &str, height: i32) -> api::SplitTreeLink {
        api::SplitTreeLink {
            child: Some(api::split_tree_link::Child::Session(api::SessionSummary {
                unique_identifier: Some(id.to_owned()),
                frame: None,
                grid_size: Some(api::Size {
                    width: Some(80),
                    height: Some(height),
                }),
                title: None,
            })),
        }
    }

    fn node(vertical: bool, links: Vec<api::SplitTreeLink>) -> api::SplitTreeNode {
        api::SplitTreeNode {
            vertical: Some(vertical),
            links,
        }
    }

    fn heights(node: &api::SplitTreeNode) -> Vec<i32> {
        node.links
            .iter()
            .flat_map(|link| match &link.child {
                Some(api::split_tree_link::Child::Session(session)) => vec![
                    session
                        .grid_size
                        .as_ref()
                        .and_then(|size| size.height)
                        .unwrap_or_default(),
                ],
                Some(api::split_tree_link::Child::Node(node)) => heights(node),
                None => vec![],
            })
            .collect()
    }

    #[test]
    fn quotes_shell_metacharacters() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote("$(rm -rf /)"), "'$(rm -rf /)'");
    }

    #[test]
    fn escapes_the_interpolated_string() {
        assert_eq!(escape_interpolation(r"a\b"), r"a\\b");
        assert_eq!(escape_interpolation(r#"a"b"#), r#"a\"b"#);
        assert_eq!(escape_interpolation(r"\(x)"), r"\\(x)");
    }

    #[test]
    fn builds_the_command_line() {
        let line = command_line(Path::new("/bin/amaca"), &["ꙮ", "ABC-1", "it's", "x/y"]);
        assert_eq!(line, "'/bin/amaca' 'ꙮ' 'ABC-1' 'it'\\\\''s' 'x/y'");
    }

    #[test]
    fn shrink_sets_the_banner_to_two_rows() {
        let mut root = node(false, vec![leaf("new", 12), leaf("old", 13)]);
        assert!(shrink(&mut root, "new"));
        assert_eq!(heights(&root), vec![2, 23]);
    }

    #[test]
    fn shrink_leaves_a_small_banner_alone() {
        let mut root = node(false, vec![leaf("new", 2), leaf("old", 5)]);
        assert!(shrink(&mut root, "new"));
        assert_eq!(heights(&root), vec![2, 5]);
    }

    #[test]
    fn shrink_propagates_through_a_stacked_sibling() {
        let sibling = node(false, vec![leaf("s1", 5), leaf("s2", 5)]);
        let mut root = node(
            false,
            vec![
                leaf("new", 12),
                api::SplitTreeLink {
                    child: Some(api::split_tree_link::Child::Node(sibling)),
                },
            ],
        );
        assert!(shrink(&mut root, "new"));
        assert_eq!(heights(&root), vec![2, 15, 5]);
    }

    #[test]
    fn shrink_propagates_through_a_side_by_side_sibling() {
        let sibling = node(true, vec![leaf("s1", 6), leaf("s2", 6)]);
        let mut root = node(
            false,
            vec![
                leaf("new", 10),
                api::SplitTreeLink {
                    child: Some(api::split_tree_link::Child::Node(sibling)),
                },
            ],
        );
        assert!(shrink(&mut root, "new"));
        assert_eq!(heights(&root), vec![2, 14, 14]);
    }
}
