#!/usr/bin/env python3
"""
Python client example for interacting with RustWeb Server

This example shows how to:
- Make GET requests
- Make POST requests with JSON data
- Handle HTTPS connections (with self-signed certificates)
- Set custom headers
- Upload files
- Stream responses
"""

import requests
import json
import urllib3
import os
from requests_toolbelt.multipart.encoder import MultipartEncoder

# Disable warnings for self-signed certificates
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

# Server configuration
SERVER_URL = "http://localhost:8080"
SERVER_HTTPS_URL = "https://localhost:8443"

def main():
    """Main function to demonstrate different API interactions"""
    print("RustWeb Server - Python Client Example")
    print("-" * 50)
    
    try:
        # Example 1: Simple GET request
        print("\n=== Example 1: Simple GET Request ===")
        response = simple_get_request()
        print(f"Status Code: {response.status_code}")
        print(f"Response Preview: {response.text[:200]}...\n")
        
        # Example 2: GET request with headers
        print("=== Example 2: GET Request with Headers ===")
        response = get_with_headers()
        print(f"Status Code: {response.status_code}")
        print(f"Response Headers: {json.dumps(dict(response.headers), indent=2)}")
        print(f"Response Preview: {response.text[:200]}...\n")
        
        # Example 3: POST request with JSON
        print("=== Example 3: POST Request with JSON ===")
        data = {
            "name": "Test User",
            "message": "Hello from Python client!"
        }
        response = post_json(data)
        print(f"Status Code: {response.status_code}")
        print(f"Response: {response.text}\n")
        
        # Example 4: HTTPS request
        print("=== Example 4: HTTPS Request ===")
        response = https_request()
        print(f"Status Code: {response.status_code}")
        print(f"Response Preview: {response.text[:200]}...\n")
        
        # Example 5: Upload file
        print("=== Example 5: File Upload ===")
        # Create a temporary test file
        with open("test_upload.txt", "w") as f:
            f.write("This is a test file for upload.\n" * 10)
        response = upload_file("test_upload.txt")
        print(f"Status Code: {response.status_code}")
        print(f"Response: {response.text}")
        # Clean up the temporary file
        os.remove("test_upload.txt")
        print()
        
        # Example 6: Stream large response
        print("=== Example 6: Stream Large Response ===")
        stream_large_response()
        
    except requests.exceptions.ConnectionError:
        print("Error: Could not connect to the server. Is it running?")
    except Exception as e:
        print(f"Error: {str(e)}")

def simple_get_request():
    """Make a simple GET request to the root URL"""
    return requests.get(f"{SERVER_URL}/")

def get_with_headers():
    """Make a GET request with custom headers"""
    headers = {
        "User-Agent": "PythonClient/1.0",
        "Accept": "text/html,application/json",
        "X-Custom-Header": "CustomValue"
    }
    return requests.get(f"{SERVER_URL}/", headers=headers)

def post_json(data):
    """Make a POST request with JSON data"""
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json"
    }
    return requests.post(f"{SERVER_URL}/api/message", headers=headers, json=data)

def https_request():
    """Make a secure HTTPS request (ignore certificate validation)"""
    return requests.get(f"{SERVER_HTTPS_URL}/", verify=False)

def upload_file(file_path):
    """Upload a file using multipart form data"""
    # Create multipart form data
    m = MultipartEncoder(
        fields={
            'field_name': 'field_value',
            'file_field': (os.path.basename(file_path), 
                          open(file_path, 'rb'), 
                          'text/plain')
        }
    )
    
    headers = {
        "Content-Type": m.content_type
    }
    
    return requests.post(f"{SERVER_URL}/upload", headers=headers, data=m)

def stream_large_response():
    """Stream a potentially large response"""
    with requests.get(f"{SERVER_URL}/large-file", stream=True) as response:
        response.raise_for_status()
        total_size = 0
        
        print("Streaming response...")
        for chunk in response.iter_content(chunk_size=8192):
            # Process the chunk
            total_size += len(chunk)
            # In a real application, you would do something with the chunk
            # For this example, we'll just count the bytes
        
        print(f"Total streamed size: {total_size} bytes")

if __name__ == "__main__":
    main() 