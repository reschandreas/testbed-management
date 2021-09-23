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
    digitalWrite(RELAYPIN, HIGH);
    delay(1000);
    digitalWrite(RELAYPIN, LOW);
    server.send(200, "text/plain", "rebooted");
  });

  server.on("/on", []() {
    digitalWrite(RELAYPIN, LOW);
    server.send(200, "text/plain", "on");
  });

  server.on("/off", []() {
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
  delay(2);
}