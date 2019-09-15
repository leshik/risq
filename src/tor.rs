use hex::encode;
use rand::Rng;
use std::convert::TryFrom;

use socks::Socks5Stream;
use socks::ToTargetAddr;
use std::fs::File;
use std::io::{self, BufRead, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

use bufstream::BufStream;

pub struct TorStream(TcpStream);

impl TorStream {
    pub fn connect(tor_proxy: SocketAddr, destination: impl ToTargetAddr) -> io::Result<TorStream> {
        Socks5Stream::connect(tor_proxy, destination).map(|stream| TorStream(stream.into_inner()))
    }
}

impl Read for TorStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for TorStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

const PROTOCOL_INFO_VERSION: i32 = 1;
const COOKIE_LENGTH: usize = 32;
const NONCE_LENGTH: usize = 32;

#[derive(Debug)]
pub enum TCErrorKind {
    ResourceExhausted,
    SyntaxErrorProtocol,
    UnrecognizedCmd,
    UnimplementedCmd,
    SyntaxErrorCmdArg,
    UnrecognizedCmdArg,
    AuthRequired,
    BadAuth,
    UnspecifiedTorError,
    InternalError,
    UnrecognizedEntity,
    InvalidConfigValue,
    InvalidDescriptor,
    UnmanagedEntity,
}
#[derive(Debug)]
pub enum TCError {
    IoError(io::Error),
    UnknownResponse,
    CannotReadAuthCookie,
    TorError(TCErrorKind),
}

impl TryFrom<u32> for TCErrorKind {
    type Error = ();
    fn try_from(code: u32) -> Result<Self, ()> {
        use TCErrorKind::*;
        match code {
            451 => Ok(ResourceExhausted),
            500 => Ok(SyntaxErrorProtocol),
            510 => Ok(UnrecognizedCmd),
            511 => Ok(UnimplementedCmd),
            512 => Ok(SyntaxErrorCmdArg),
            513 => Ok(UnrecognizedCmdArg),
            514 => Ok(AuthRequired),
            515 => Ok(BadAuth),
            550 => Ok(UnspecifiedTorError),
            551 => Ok(InternalError),
            552 => Ok(UnrecognizedEntity),
            553 => Ok(InvalidConfigValue),
            554 => Ok(InvalidDescriptor),
            555 => Ok(UnmanagedEntity),
            _ => Err(()),
        }
    }
}

impl From<io::Error> for TCError {
    fn from(err: io::Error) -> Self {
        TCError::IoError(err)
    }
}

type TCResult<T> = Result<T, TCError>;

pub struct TorControl(BufStream<TcpStream>);

#[derive(Debug)]
pub struct ProtocolInfo {
    pub cookiefile: String,
    pub auth_methods: Vec<String>,
    pub tor_version: String,
}

impl TorControl {
    pub fn connect<A: ToSocketAddrs>(addr: A) -> TCResult<Self> {
        let mut tc = TorControl(BufStream::new(TcpStream::connect(addr)?));
        println!("{:?}", tc.protocol_info());
        println!("{:?}", tc.authenticate());
        Ok(tc)
    }

    pub fn protocol_info(&mut self) -> TCResult<ProtocolInfo> {
        send_command(
            &mut self.0,
            format!("PROTOCOLINFO {}", PROTOCOL_INFO_VERSION).into(),
        )?;
        let response = read_lines(&mut self.0)?.join(" ");
        let mut cookiefile = "";
        let mut auth_methods = "";
        let mut tor_version = "";
        for section in response.split(" ") {
            let split: Vec<&str> = section.split("=").collect();
            if split.len() == 2 {
                match split[0] {
                    "COOKIEFILE" => cookiefile = split[1],
                    "METHODS" => auth_methods = split[1],
                    "Tor" => tor_version = split[1],
                    _ => (),
                }
            }
        }
        Ok(ProtocolInfo {
            cookiefile: cookiefile.trim_matches('\"').into(),
            auth_methods: auth_methods.split(",").map(|s| s.into()).collect(),
            tor_version: tor_version.trim_matches('\"').into(),
        })
    }

    fn authenticate(&mut self) -> TCResult<String> {
        let random_bytes = rand::thread_rng().gen::<[u8; NONCE_LENGTH]>();
        send_command(
            &mut self.0,
            format!("AUTHCHALLENGE SAFECOOKIE {}", hex::encode(random_bytes)),
        )?;
        Ok(read_lines(&mut self.0)?.join(" "))
        // clientNonce := make([]byte, nonceLen)
        // if _, err := rand.Read(clientNonce); err != nil {
        // return fmt.Errorf("unable to generate client nonce: %v", err)
        // }
        // cmd := fmt.Sprintf("AUTHCHALLENGE SAFECOOKIE %x", clientNonce)_, reply, err := c.sendCommand(cmd)if err != nil {return err
        // }
    }

    fn get_auth_cookie(&mut self) -> TCResult<Vec<u8>> {
        let info = self.protocol_info()?;
        println!("Ainfo is here {:?}", info);
        let mut file_content = Vec::new();
        let length = File::open(info.cookiefile)?.read_to_end(&mut file_content)?;
        if length != COOKIE_LENGTH {
            Err(TCError::CannotReadAuthCookie)
        } else {
            Ok(file_content)
        }
    }
}

pub trait AuthenticatedTorControl {}

fn send_command<W: Write>(writer: &mut W, command: String) -> Result<(), io::Error> {
    println!("sending command: '{}'", command);
    write!(writer, "{}\r\n", command)?;
    writer.flush()
}

fn is_last_line(line: &str) -> TCResult<bool> {
    // Act upon separator:
    match &line[3..4] {
        // Meaning: this is the last line to read.
        " " => Ok(true),
        // We have more lines to read.
        "+" | "-" => Ok(false),
        _ => Err(TCError::UnknownResponse),
    }
}

fn parse_status(line: &str) -> TCResult<u32> {
    (&line[0..3]).parse().map_err(|_| TCError::UnknownResponse)
}

fn parse_line<'b, R: BufRead>(
    stream: &mut R,
    buf: &'b mut String,
) -> TCResult<(u32, bool, &'b str)> {
    // Read a line and make sure we have at least 3 (status) + 1 (sep) bytes.
    if stream.read_line(buf)? < 4 {
        return Err(TCError::UnknownResponse);
    }
    let (buf_s, msg) = buf.split_at(4);
    let status = parse_status(&buf_s)?;
    let is_last_line = is_last_line(&buf_s)?;
    Ok((status, is_last_line, msg))
}
fn read_lines<R: BufRead>(read: &mut R) -> TCResult<Vec<String>> {
    let mut rls: Vec<String> = Vec::with_capacity(1);
    let mut buf = String::new();
    loop {
        {
            let (status, end, msg) = parse_line(read, &mut buf)?;
            handle_code(status)?;
            rls.push(msg.trim_end().to_owned());
            if end {
                break;
            }
        }
        buf.clear();
    }

    Ok(rls)
}

fn handle_code(status: u32) -> TCResult<()> {
    use TCError::*;
    match status {
        250 | 251 => Ok(()),
        status => Err(TCErrorKind::try_from(status)
            .map(TorError)
            .unwrap_or(UnknownResponse)),
    }
}

#[cfg(test)]
mod tests {

    use super::TorStream;
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    #[test]
    fn check_clear_web() -> std::io::Result<()> {
        let address = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9050));
        let mut stream = TorStream::connect(address, "www.example.com:80")?;

        stream
            .write_all(b"GET / HTTP/1.1\r\nConnection: Close\r\nHost: www.example.com\r\n\r\n")
            .expect("Failed to send request");

        let mut buf = String::with_capacity(1633);
        stream
            .read_to_string(&mut buf)
            .expect("Failed to read response");

        assert!(buf.starts_with("HTTP/1.1 200 OK"));
        Ok(())
    }

    #[test]
    fn check_hidden_service() -> std::io::Result<()> {
        let address = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9050));
        let mut stream = TorStream::connect(
            address,
            (
                "p53lf57qovyuvwsc6xnrppyply3vtqm7l6pcobkmyqsiofyeznfu5uqd.onion",
                80,
            ),
        )?;

        stream
            .write_all(b"GET / HTTP/1.1\r\nConnection: Close\r\nHost: p53lf57qovyuvwsc6xnrppyply3vtqm7l6pcobkmyqsiofyeznfu5uqd.onion\r\n\r\n")
            .expect("Failed to send request");

        let mut buf = String::with_capacity(390);
        stream
            .read_to_string(&mut buf)
            .expect("Failed to read response");

        assert!(buf.starts_with("HTTP/1.1 302"));
        Ok(())
    }
}
