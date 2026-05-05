use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct LoadClientConfig {
    pub base_url: String,
    pub iterations: usize,
    pub parallelism: usize,
    pub name_prefix: String,
    pub rows: usize,
    pub cols: usize,
    pub variables_per_part: usize,
}

impl Default for LoadClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:8000".to_string(),
            iterations: 5000,
            parallelism: 32,
            name_prefix: "rust-stress-case".to_string(),
            rows: 256,
            cols: 256,
            variables_per_part: 3,
        }
    }
}

#[derive(Debug)]
pub struct LoadClientSummary {
    pub iterations: usize,
    pub parallelism: usize,
    pub elapsed: std::time::Duration,
}

impl LoadClientSummary {
    pub fn cases_per_second(&self) -> f64 {
        self.iterations as f64 / self.elapsed.as_secs_f64()
    }

    pub fn requests_per_second(&self) -> f64 {
        (self.iterations * 3) as f64 / self.elapsed.as_secs_f64()
    }
}

pub fn run(config: LoadClientConfig) -> io::Result<LoadClientSummary> {
    if config.parallelism == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "parallelism must be greater than zero",
        ));
    }
    if config.rows == 0 || config.cols == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "rows and cols must be greater than zero",
        ));
    }
    if config.variables_per_part == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "variables per part must be greater than zero",
        ));
    }

    let endpoint = Arc::new(HttpEndpoint::parse(&config.base_url)?);
    let part_a = Arc::new(create_part_bytes(
        "part-a",
        "max_temp",
        "temperature",
        config.rows,
        config.cols,
        config.variables_per_part,
        1.0,
    )?);
    let part_b = Arc::new(create_part_bytes(
        "part-b",
        "avg_humidity",
        "humidity",
        config.rows,
        config.cols,
        config.variables_per_part,
        10.0,
    )?);
    let jobs = Arc::new(Mutex::new((0..config.iterations).collect::<VecDeque<_>>()));
    let start = Instant::now();

    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(config.parallelism);
        for _ in 0..config.parallelism {
            let endpoint = Arc::clone(&endpoint);
            let part_a = Arc::clone(&part_a);
            let part_b = Arc::clone(&part_b);
            let jobs = Arc::clone(&jobs);
            let name_prefix = config.name_prefix.clone();

            handles.push(scope.spawn(move || -> io::Result<()> {
                loop {
                    let Some(index) = jobs.lock().expect("jobs mutex poisoned").pop_front() else {
                        return Ok(());
                    };
                    let name = format!("{name_prefix}-{index}");
                    upload(&endpoint, "part_a", &name, &part_a)?;
                    upload(&endpoint, "part_b", &name, &part_b)?;
                    let response = request(&endpoint, "GET", &format!("/read?name={name}"), None)?;
                    if response.status != 200 {
                        return Err(io::Error::other(format!(
                            "GET /read failed for {name}: HTTP {}",
                            response.status
                        )));
                    }
                }
            }));
        }

        for handle in handles {
            handle
                .join()
                .map_err(|_| io::Error::other("load client thread panicked"))??;
        }

        Ok::<(), io::Error>(())
    })?;

    Ok(LoadClientSummary {
        iterations: config.iterations,
        parallelism: config.parallelism,
        elapsed: start.elapsed(),
    })
}

fn upload(endpoint: &HttpEndpoint, part: &str, name: &str, body: &[u8]) -> io::Result<()> {
    let response = request(
        endpoint,
        "POST",
        &format!("/{part}?name={name}"),
        Some(body),
    )?;
    if response.status != 200 {
        return Err(io::Error::other(format!(
            "POST /{part} failed for {name}: HTTP {}",
            response.status
        )));
    }
    Ok(())
}

fn create_part_bytes(
    title: &str,
    attribute_name: &str,
    variable_prefix: &str,
    rows: usize,
    cols: usize,
    variables_per_part: usize,
    value_offset: f32,
) -> io::Result<Vec<u8>> {
    let mut dataset = netcdf::create_mem(1024 * 1024).map_err(to_io_error)?;
    dataset.add_dimension("lat", rows).map_err(to_io_error)?;
    dataset.add_dimension("lon", cols).map_err(to_io_error)?;
    dataset.add_attribute("title", title).map_err(to_io_error)?;
    dataset
        .add_attribute(attribute_name, "1")
        .map_err(to_io_error)?;

    for variable_index in 0..variables_per_part {
        let variable_name = format!("{variable_prefix}_{variable_index}");
        let mut variable = dataset
            .add_variable::<f32>(&variable_name, &["lat", "lon"])
            .map_err(to_io_error)?;
        variable.put_attribute("units", "1").map_err(to_io_error)?;
    }

    dataset.enddef().map_err(to_io_error)?;

    let values = create_values(rows, cols, value_offset);
    for variable_index in 0..variables_per_part {
        let variable_name = format!("{variable_prefix}_{variable_index}");
        let mut variable = dataset
            .variable_mut(&variable_name)
            .ok_or_else(|| io::Error::other("created variable was not found"))?;
        variable
            .put_values(&values, (.., ..))
            .map_err(to_io_error)?;
    }

    dataset.close_to_bytes().map_err(to_io_error)
}

fn create_values(rows: usize, cols: usize, offset: f32) -> Vec<f32> {
    (0..rows * cols)
        .map(|index| offset + (index % cols) as f32 * 0.01 + (index / cols) as f32 * 0.001)
        .collect()
}

fn to_io_error<E: std::fmt::Display>(err: E) -> io::Error {
    io::Error::other(err.to_string())
}

#[derive(Debug)]
struct HttpEndpoint {
    host: String,
    port: u16,
}

impl HttpEndpoint {
    fn parse(base_url: &str) -> io::Result<Self> {
        let Some(rest) = base_url.strip_prefix("http://") else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "only http:// URLs are supported",
            ));
        };
        let authority = rest.split('/').next().unwrap_or(rest);
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => {
                let port = port.parse::<u16>().map_err(|err| {
                    io::Error::new(io::ErrorKind::InvalidInput, format!("invalid port: {err}"))
                })?;
                (host.to_string(), port)
            }
            None => (authority.to_string(), 80),
        };

        if host.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "URL host must not be empty",
            ));
        }

        Ok(Self { host, port })
    }

    fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    _body: Vec<u8>,
}

fn request(
    endpoint: &HttpEndpoint,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> io::Result<HttpResponse> {
    let mut stream = TcpStream::connect(endpoint.addr())?;
    let body_len = body.map_or(0, <[u8]>::len);
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {body_len}\r\n",
        endpoint.host
    )?;
    if body.is_some() {
        write!(stream, "Content-Type: application/netcdf\r\n")?;
    }
    write!(stream, "\r\n")?;
    if let Some(body) = body {
        stream.write_all(body)?;
    }
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;
    let status = parse_status(&status_line)?;

    let mut content_length = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }

    let mut body = Vec::new();
    if let Some(content_length) = content_length {
        body.resize(content_length, 0);
        reader.read_exact(&mut body)?;
    } else {
        reader.read_to_end(&mut body)?;
    }

    Ok(HttpResponse {
        status,
        _body: body,
    })
}

fn parse_status(status_line: &str) -> io::Result<u16> {
    status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| io::Error::other(format!("invalid HTTP response: {status_line:?}")))?
        .parse::<u16>()
        .map_err(|err| io::Error::other(format!("invalid HTTP status: {err}")))
}
