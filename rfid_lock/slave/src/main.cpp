#include <Arduino.h>
#include <Wire.h>

constexpr uint8_t I2C_SDA_PIN = 32;
constexpr uint8_t I2C_SCL_PIN = 33;
constexpr uint8_t I2C_SLAVE_ADDRESS = 0x08;
constexpr uint8_t SOLENOID_PIN = 25;  // Solenoid lock pin

auto receiveEvent(int numberOfBytes) -> void;

auto setup() -> void {
    Serial.begin(9600);
    Wire.begin(I2C_SLAVE_ADDRESS, I2C_SDA_PIN, I2C_SCL_PIN, 100);
    Wire.onReceive(receiveEvent);
    pinMode(SOLENOID_PIN, OUTPUT);  // Set solenoid pin as output
    digitalWrite(SOLENOID_PIN, LOW);  // Lock the door
}

auto loop() -> void {
    delay(100);
}

bool prev = false;

auto receiveEvent(int numberOfBytes) -> void {
    digitalWrite(SOLENOID_PIN, prev ? HIGH : LOW);
    prev = !prev;
//    if (Wire.available()) {
//        uint8_t value = Wire.read();
//
//        if (value == '1') {
//            digitalWrite(SOLENOID_PIN, HIGH);
//        } else {
//            digitalWrite(SOLENOID_PIN, LOW);
//        }
//    }
}