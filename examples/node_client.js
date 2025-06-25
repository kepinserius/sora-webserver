#!/usr/bin/env node

/**
 * Node.js client example for interacting with RustWeb Server
 * 
 * This example demonstrates:
 * - Making HTTP and HTTPS requests
 * - Handling JSON data
 * - Using custom headers
 * - File uploads
 * - Streaming responses
 */

const http = require('http');
const https = require('https');
const fs = require('fs');
const path = require('path');
const FormData = require('form-data');

// Server configuration
const SERVER_HOST = 'localhost';
const HTTP_PORT = 8080;
const HTTPS_PORT = 8443;

// Disable certificate validation for self-signed certs (development only)
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

/**
 * Main function to run all examples
 */
async function main() {
  console.log('RustWeb Server - Node.js Client Example');
  console.log('-'.repeat(50));
  
  try {
    // Example 1: Simple HTTP GET request
    console.log('\n=== Example 1: Simple HTTP GET Request ===');
    const getResponse = await makeGetRequest('/');
    console.log(`Status: ${getResponse.statusCode}`);
    console.log(`Response preview: ${getResponse.body.substring(0, 200)}...\n`);
    
    // Example 2: GET with custom headers
    console.log('=== Example 2: GET with Custom Headers ===');
    const headers = {
      'User-Agent': 'NodeClient/1.0',
      'Accept': 'text/html,application/json',
      'X-Custom-Header': 'CustomValue'
    };
    const headerResponse = await makeGetRequest('/', headers);
    console.log(`Status: ${headerResponse.statusCode}`);
    console.log('Response headers:', headerResponse.headers);
    console.log(`Response preview: ${headerResponse.body.substring(0, 200)}...\n`);
    
    // Example 3: POST JSON data
    console.log('=== Example 3: POST JSON Data ===');
    const postData = {
      name: 'Test User',
      message: 'Hello from Node.js client!'
    };
    const postResponse = await makePostRequest('/api/message', postData);
    console.log(`Status: ${postResponse.statusCode}`);
    console.log(`Response: ${postResponse.body}\n`);
    
    // Example 4: HTTPS request
    console.log('=== Example 4: HTTPS Request ===');
    const httpsResponse = await makeHttpsRequest('/');
    console.log(`Status: ${httpsResponse.statusCode}`);
    console.log(`Response preview: ${httpsResponse.body.substring(0, 200)}...\n`);
    
    // Example 5: File upload
    console.log('=== Example 5: File Upload ===');
    // Create a test file
    fs.writeFileSync('test_upload.txt', 'This is a test file for upload.\n'.repeat(10));
    const uploadResponse = await uploadFile('test_upload.txt', '/upload');
    console.log(`Status: ${uploadResponse.statusCode}`);
    console.log(`Response: ${uploadResponse.body}`);
    // Clean up
    fs.unlinkSync('test_upload.txt');
    console.log();
    
    // Example 6: Stream response
    console.log('=== Example 6: Stream Response ===');
    await streamResponse('/large-file');
    
  } catch (error) {
    console.error('Error:', error.message);
  }
}

/**
 * Make a GET request to the server
 */
function makeGetRequest(path, headers = {}) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: SERVER_HOST,
      port: HTTP_PORT,
      path: path,
      method: 'GET',
      headers: headers
    };
    
    const req = http.request(options, (res) => {
      let data = '';
      
      res.on('data', (chunk) => {
        data += chunk;
      });
      
      res.on('end', () => {
        resolve({
          statusCode: res.statusCode,
          headers: res.headers,
          body: data
        });
      });
    });
    
    req.on('error', (error) => {
      reject(error);
    });
    
    req.end();
  });
}

/**
 * Make a POST request with JSON data
 */
function makePostRequest(path, jsonData) {
  return new Promise((resolve, reject) => {
    const data = JSON.stringify(jsonData);
    
    const options = {
      hostname: SERVER_HOST,
      port: HTTP_PORT,
      path: path,
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': data.length
      }
    };
    
    const req = http.request(options, (res) => {
      let responseData = '';
      
      res.on('data', (chunk) => {
        responseData += chunk;
      });
      
      res.on('end', () => {
        resolve({
          statusCode: res.statusCode,
          headers: res.headers,
          body: responseData
        });
      });
    });
    
    req.on('error', (error) => {
      reject(error);
    });
    
    req.write(data);
    req.end();
  });
}

/**
 * Make an HTTPS request
 */
function makeHttpsRequest(path) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: SERVER_HOST,
      port: HTTPS_PORT,
      path: path,
      method: 'GET'
    };
    
    const req = https.request(options, (res) => {
      let data = '';
      
      res.on('data', (chunk) => {
        data += chunk;
      });
      
      res.on('end', () => {
        resolve({
          statusCode: res.statusCode,
          headers: res.headers,
          body: data
        });
      });
    });
    
    req.on('error', (error) => {
      reject(error);
    });
    
    req.end();
  });
}

/**
 * Upload a file using multipart form data
 */
function uploadFile(filePath, endpoint) {
  return new Promise((resolve, reject) => {
    const form = new FormData();
    form.append('field_name', 'field_value');
    form.append('file_field', fs.createReadStream(filePath));
    
    const options = {
      hostname: SERVER_HOST,
      port: HTTP_PORT,
      path: endpoint,
      method: 'POST',
      headers: form.getHeaders()
    };
    
    const req = http.request(options, (res) => {
      let responseData = '';
      
      res.on('data', (chunk) => {
        responseData += chunk;
      });
      
      res.on('end', () => {
        resolve({
          statusCode: res.statusCode,
          headers: res.headers,
          body: responseData
        });
      });
    });
    
    req.on('error', (error) => {
      reject(error);
    });
    
    form.pipe(req);
  });
}

/**
 * Stream a large response
 */
function streamResponse(path) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: SERVER_HOST,
      port: HTTP_PORT,
      path: path,
      method: 'GET'
    };
    
    const req = http.request(options, (res) => {
      let totalSize = 0;
      
      console.log('Streaming response...');
      
      res.on('data', (chunk) => {
        // Process each chunk of data
        totalSize += chunk.length;
        // In a real application, you would do something with each chunk
      });
      
      res.on('end', () => {
        console.log(`Total streamed size: ${totalSize} bytes`);
        resolve();
      });
    });
    
    req.on('error', (error) => {
      reject(error);
    });
    
    req.end();
  });
}

// Run the examples
main().catch(console.error); 