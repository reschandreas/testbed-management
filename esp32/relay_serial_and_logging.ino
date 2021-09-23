#include <WiFi.h>
#include <ESPmDNS.h>
#include <ArduinoOTA.h>
#include <WebServer.h>
#include <HTTPClient.h>

const char* ssid = "test-cluster";
const char* password = "uLEnKSkdCz";

WebServer server(80);

const char* www_username = "admin";
const char* www_password = "esp32";
HardwareSerial SBCSerial(1);
String to_discard = "";

#define RELAYPIN 13

void setup() {
  Serial.begin(115200);
  //https://www.espressif.com/sites/default/files/documentation/esp32-wroom-32_datasheet_en.pdf
  //https://github.com/espressif/arduino-esp32/blob/master/cores/esp32/HardwareSerial.cpp
  SBCSerial.begin(115200, SERIAL_8N1, 16, 17);
  WiFi.mode(WIFI_STA);
  pinMode(RELAYPIN, OUTPUT);
  WiFi.begin(ssid, password);
  if (WiFi.waitForConnectResult() != WL_CONNECTED) {
    Serial.println("WiFi Connect Failed! Rebooting...");
    delay(1000);
    ESP.restart();
  }
  ArduinoOTA.begin();
  server.on("/", []() {
    if (!server.authenticate(www_username, www_password)) {
      return server.requestAuthentication();
    }
    digitalWrite(RELAYPIN, HIGH);
    delay(1000);
    digitalWrite(RELAYPIN, LOW);
    server.send(200, "text/plain", "rebooted");
  });

  server.on("/on", []() {
    if (!server.authenticate(www_username, www_password)) {
      return server.requestAuthentication();
    }
    digitalWrite(RELAYPIN, LOW);
    server.send(200, "text/plain", "on");
  });

  server.on("/off", []() {
    if (!server.authenticate(www_username, www_password)) {
      return server.requestAuthentication();
    }
    digitalWrite(RELAYPIN, HIGH);
    server.send(200, "text/plain", "off");
  });
  server.begin();

  Serial.print("Open http://");
  Serial.print(WiFi.localIP());
  Serial.println("/ in your browser to see it working!");
}

void loop() {
  ArduinoOTA.handle();
  server.handleClient();
  delay(2);//allow the cpu to switch to other tasks
  if (SBCSerial.available() > 0) {
    String str = SBCSerial.readString();
    Serial.print(str);
    str.replace("\r\n", "\n");
    HTTPClient http;
    http.begin("http://10.0.0.53:8080/");
    int httpResponseCode = http.POST(str);
    //Serial.print("HTTP Response code: ");
    //Serial.println(httpResponseCode);
  }
  if (Serial.available() > 0) {
    String str = Serial.readString();
    SBCSerial.print(str);
  }
  delay(2);
}