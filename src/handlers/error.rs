use hyper::{Body, Response, StatusCode, header};

// Create a simple HTML error page
fn html_error_page(status: StatusCode, message: &str) -> String {
    let status_code = status.as_u16();
    let status_text = status.canonical_reason().unwrap_or("Error");
    
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} {}</title>
    <style>
        body {{
            font-family: Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 650px;
            margin: 0 auto;
            padding: 1rem;
        }}
        .error-container {{
            margin-top: 3rem;
            border: 1px solid #ddd;
            border-radius: 4px;
            padding: 2rem;
            text-align: center;
        }}
        h1 {{
            margin-top: 0;
            color: #e74c3c;
        }}
        .status {{
            font-size: 1.2rem;
            color: #7f8c8d;
            margin-bottom: 1.5rem;
        }}
        .message {{
            font-size: 1.1rem;
        }}
        .footer {{
            margin-top: 2rem;
            font-size: 0.8rem;
            color: #95a5a6;
        }}
    </style>
</head>
<body>
    <div class="error-container">
        <h1>{} {}</h1>
        <div class="status">Status Code: {}</div>
        <div class="message">{}</div>
        <div class="footer">Rust Web Server</div>
    </div>
</body>
</html>"#,
        status_code, status_text, status_code, status_text, status_code, message
    )
}

// Create an error response with the given status code and message
fn error_response(status: StatusCode, message: &str) -> Response<Body> {
    let body = html_error_page(status, message);
    
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;
    
    // Set content type
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/html; charset=utf-8"),
    );
    
    response
}

// 400 Bad Request
pub fn bad_request() -> Response<Body> {
    error_response(
        StatusCode::BAD_REQUEST,
        "The server could not understand the request due to invalid syntax.",
    )
}

// 401 Unauthorized
pub fn unauthorized() -> Response<Body> {
    let mut response = error_response(
        StatusCode::UNAUTHORIZED,
        "Authentication is required and has failed or has not been provided.",
    );
    
    // Add WWW-Authenticate header
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        header::HeaderValue::from_static("Basic realm=\"restricted\""),
    );
    
    response
}

// 403 Forbidden
pub fn forbidden() -> Response<Body> {
    error_response(
        StatusCode::FORBIDDEN,
        "You don't have permission to access this resource.",
    )
}

// 404 Not Found
pub fn not_found() -> Response<Body> {
    error_response(
        StatusCode::NOT_FOUND,
        "The requested resource could not be found.",
    )
}

// 405 Method Not Allowed
pub fn method_not_allowed(allowed: &[&str]) -> Response<Body> {
    let mut response = error_response(
        StatusCode::METHOD_NOT_ALLOWED,
        "The request method is not supported for this resource.",
    );
    
    // Add Allow header
    let allow = allowed.join(", ");
    response.headers_mut().insert(
        header::ALLOW,
        header::HeaderValue::from_str(&allow)
            .unwrap_or_else(|_| header::HeaderValue::from_static("GET, HEAD")),
    );
    
    response
}

// 408 Request Timeout
pub fn request_timeout() -> Response<Body> {
    error_response(
        StatusCode::REQUEST_TIMEOUT,
        "The server timed out waiting for the request.",
    )
}

// 413 Payload Too Large
pub fn payload_too_large() -> Response<Body> {
    error_response(
        StatusCode::PAYLOAD_TOO_LARGE,
        "The request entity is larger than limits defined by server.",
    )
}

// 429 Too Many Requests
pub fn too_many_requests(retry_after: u64) -> Response<Body> {
    let mut response = error_response(
        StatusCode::TOO_MANY_REQUESTS,
        "Too many requests, please try again later.",
    );
    
    // Add Retry-After header
    response.headers_mut().insert(
        header::RETRY_AFTER,
        header::HeaderValue::from(retry_after),
    );
    
    response
}

// 500 Internal Server Error
pub fn internal_server_error() -> Response<Body> {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "The server encountered an internal error and was unable to complete your request.",
    )
}

// 502 Bad Gateway
pub fn bad_gateway() -> Response<Body> {
    error_response(
        StatusCode::BAD_GATEWAY,
        "The server received an invalid response from an upstream server.",
    )
}

// 503 Service Unavailable
pub fn service_unavailable(retry_after: Option<u64>) -> Response<Body> {
    let mut response = error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "The server is currently unavailable. Please try again later.",
    );
    
    // Add Retry-After header if provided
    if let Some(seconds) = retry_after {
        response.headers_mut().insert(
            header::RETRY_AFTER,
            header::HeaderValue::from(seconds),
        );
    }
    
    response
}

// 504 Gateway Timeout
pub fn gateway_timeout() -> Response<Body> {
    error_response(
        StatusCode::GATEWAY_TIMEOUT,
        "The server was acting as a gateway or proxy and did not receive a timely response from the upstream server.",
    )
} 