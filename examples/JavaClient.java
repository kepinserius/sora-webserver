import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.security.cert.X509Certificate;
import javax.net.ssl.HostnameVerifier;
import javax.net.ssl.HttpsURLConnection;
import javax.net.ssl.SSLContext;
import javax.net.ssl.TrustManager;
import javax.net.ssl.X509TrustManager;

/**
 * Java client example for interacting with RustWeb Server
 * 
 * This example shows how to:
 * - Make GET requests
 * - Make POST requests with JSON data
 * - Handle HTTPS connections (with self-signed certificates)
 * - Set custom headers
 */
public class JavaClient {

    private static final String SERVER_URL = "http://localhost:8080";
    private static final String SERVER_HTTPS_URL = "https://localhost:8443";

    public static void main(String[] args) {
        try {
            // For development with self-signed certificates
            disableCertificateValidation();

            // Example 1: Simple GET request
            System.out.println("=== Example 1: Simple GET Request ===");
            String response = sendGetRequest(SERVER_URL + "/");
            System.out.println("Response:\n" + response.substring(0, Math.min(200, response.length())) + "...\n");

            // Example 2: GET request with headers
            System.out.println("=== Example 2: GET Request with Headers ===");
            String customHeaderResponse = sendGetRequestWithHeaders(SERVER_URL + "/");
            System.out.println("Response with custom headers:\n" + 
                customHeaderResponse.substring(0, Math.min(200, customHeaderResponse.length())) + "...\n");

            // Example 3: POST request with JSON
            System.out.println("=== Example 3: POST Request with JSON ===");
            String jsonPayload = "{\"name\":\"Test User\",\"message\":\"Hello from Java client!\"}";
            String postResponse = sendPostRequest(SERVER_URL + "/api/message", jsonPayload);
            System.out.println("POST Response:\n" + postResponse + "\n");

            // Example 4: HTTPS request (secure)
            System.out.println("=== Example 4: HTTPS Request ===");
            String secureResponse = sendGetRequest(SERVER_HTTPS_URL + "/");
            System.out.println("Secure Response:\n" + 
                secureResponse.substring(0, Math.min(200, secureResponse.length())) + "...\n");

        } catch (Exception e) {
            System.err.println("Error: " + e.getMessage());
            e.printStackTrace();
        }
    }

    /**
     * Send a simple GET request to the specified URL
     */
    private static String sendGetRequest(String urlStr) throws Exception {
        URL url = new URL(urlStr);
        HttpURLConnection conn = (HttpURLConnection) url.openConnection();
        
        conn.setRequestMethod("GET");
        conn.setConnectTimeout(5000);
        conn.setReadTimeout(5000);
        
        int status = conn.getResponseCode();
        
        BufferedReader in = new BufferedReader(
            new InputStreamReader(conn.getInputStream()));
        String inputLine;
        StringBuilder content = new StringBuilder();
        while ((inputLine = in.readLine()) != null) {
            content.append(inputLine).append("\n");
        }
        in.close();
        
        System.out.println("Status: " + status);
        conn.disconnect();
        
        return content.toString();
    }
    
    /**
     * Send a GET request with custom headers
     */
    private static String sendGetRequestWithHeaders(String urlStr) throws Exception {
        URL url = new URL(urlStr);
        HttpURLConnection conn = (HttpURLConnection) url.openConnection();
        
        conn.setRequestMethod("GET");
        conn.setConnectTimeout(5000);
        conn.setReadTimeout(5000);
        
        // Set custom headers
        conn.setRequestProperty("User-Agent", "JavaClient/1.0");
        conn.setRequestProperty("Accept", "text/html,application/json");
        conn.setRequestProperty("X-Custom-Header", "CustomValue");
        
        int status = conn.getResponseCode();
        
        BufferedReader in = new BufferedReader(
            new InputStreamReader(conn.getInputStream()));
        String inputLine;
        StringBuilder content = new StringBuilder();
        while ((inputLine = in.readLine()) != null) {
            content.append(inputLine).append("\n");
        }
        in.close();
        
        System.out.println("Status: " + status);
        conn.disconnect();
        
        return content.toString();
    }
    
    /**
     * Send a POST request with JSON payload
     */
    private static String sendPostRequest(String urlStr, String jsonPayload) throws Exception {
        URL url = new URL(urlStr);
        HttpURLConnection conn = (HttpURLConnection) url.openConnection();
        
        conn.setRequestMethod("POST");
        conn.setConnectTimeout(5000);
        conn.setReadTimeout(5000);
        conn.setDoOutput(true);
        
        // Set JSON headers
        conn.setRequestProperty("Content-Type", "application/json");
        conn.setRequestProperty("Accept", "application/json");
        
        // Write JSON data
        try (OutputStream os = conn.getOutputStream()) {
            byte[] input = jsonPayload.getBytes(StandardCharsets.UTF_8);
            os.write(input, 0, input.length);
        }
        
        int status = conn.getResponseCode();
        System.out.println("Status: " + status);
        
        BufferedReader in;
        if (status >= 200 && status < 300) {
            in = new BufferedReader(new InputStreamReader(conn.getInputStream()));
        } else {
            in = new BufferedReader(new InputStreamReader(conn.getErrorStream()));
        }
        
        String inputLine;
        StringBuilder content = new StringBuilder();
        while ((inputLine = in.readLine()) != null) {
            content.append(inputLine).append("\n");
        }
        in.close();
        
        conn.disconnect();
        
        return content.toString();
    }
    
    /**
     * Disable certificate validation for development with self-signed certificates
     * NOTE: Do not use this in production!
     */
    private static void disableCertificateValidation() throws Exception {
        // Create a trust manager that accepts all certificates
        TrustManager[] trustAllCerts = new TrustManager[] {
            new X509TrustManager() {
                public X509Certificate[] getAcceptedIssuers() {
                    return null;
                }
                
                public void checkClientTrusted(X509Certificate[] certs, String authType) {
                }
                
                public void checkServerTrusted(X509Certificate[] certs, String authType) {
                }
            }
        };
        
        // Create SSL context and use the trust manager
        SSLContext sc = SSLContext.getInstance("TLS");
        sc.init(null, trustAllCerts, new java.security.SecureRandom());
        HttpsURLConnection.setDefaultSSLSocketFactory(sc.getSocketFactory());
        
        // Create all-trusting host name verifier
        HostnameVerifier allHostsValid = (hostname, session) -> true;
        HttpsURLConnection.setDefaultHostnameVerifier(allHostsValid);
    }
} 