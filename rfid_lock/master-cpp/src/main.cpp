#include <Arduino.h>
#include <SPI.h>
#include <MFRC522.h>
#include <Wire.h>
#include <ESP32Servo.h>

constexpr auto SS_PIN = GPIO_NUM_5; // SDA (Slave Select) pin of RFID module
constexpr auto RST_PIN = GPIO_NUM_13; // Reset pin of RFID module
constexpr auto SOLENOID_PIN = GPIO_NUM_14;
constexpr auto SERVO_PIN = GPIO_NUM_12;
constexpr auto AUTHORIZED_LED_PIN = GPIO_NUM_17;
constexpr auto UNAUTHORIZED_LED_PIN = GPIO_NUM_16;
constexpr auto BUZZER_PIN = GPIO_NUM_27;
constexpr auto I2C_SDA_PIN = GPIO_NUM_26;
constexpr auto I2C_SCL_PIN = GPIO_NUM_25;
constexpr auto I2C_SLAVE_ADDRESS = 0x00;


MFRC522 rfid(SS_PIN, RST_PIN); // Create RFID instance

// List of authorized RFID card UIDs
byte authorizedCards[][4] = {
    {0xE3, 0x95, 0x0C, 0x0E}, // Card UID 1 {0x9A, 0xBC, 0xDE, 0xF0} // Card UID 2
    {0x13, 0x85, 0x07, 0x15}, // Card UID 1 {0x9A, 0xBC, 0xDE, 0xF0} // Card UID 2
};

Servo servo;

auto isAuthorizedCard(const byte *uid) -> bool;

auto unlockDoor(uint8_t solenoidPin, uint8_t authorizedLedPin,
                uint8_t solenoidI2cAddr = 0,
                Servo *servo = nullptr) -> void;

auto playUnauthorizedCardMelody(uint64_t buzzerPin) -> void;

auto playAuthorizedCardBeep(uint64_t buzzerPin) -> void;

auto sweep(Servo *servo, int start, int end, uint8_t step = 1,
           uint8_t delay_ms = 1) -> void;

auto sendI2Uint8(uint8_t data, uint8_t address) -> void {
    Wire.beginTransmission(address);
    Wire.write(data);
    Wire.endTransmission();
}

auto setup() -> void {
    Serial.begin(9600);
    SPI.begin();
    rfid.PCD_Init(); // Initialize RFID reader
    pinMode(AUTHORIZED_LED_PIN, OUTPUT);
    pinMode(UNAUTHORIZED_LED_PIN, OUTPUT);
    pinMode(BUZZER_PIN, OUTPUT);
    pinMode(SOLENOID_PIN, OUTPUT);
    Wire.begin(I2C_SDA_PIN, I2C_SCL_PIN);
    servo.attach(SERVO_PIN);
    servo.write(0);
}

auto loop() -> void {
    auto isCardPresent = rfid.PICC_IsNewCardPresent();
    auto isCardRead = rfid.PICC_ReadCardSerial();

    if (isCardPresent && isCardRead) {
        // new tag is available
        // NUID has been readed
        auto piccType = MFRC522::PICC_GetType(rfid.uid.sak);
        Serial.print("RFID/NFC Tag Type: ");
        Serial.println(MFRC522::PICC_GetTypeName(piccType));

        // print NUID in Serial Monitor in the hex format
        Serial.print("UID:");
        auto uidByte = rfid.uid.uidByte;
        auto uidSize = rfid.uid.size;
        for (int i = 0; i < uidSize; i++) {
            Serial.print(uidByte[i] < 0x10 ? " 0" : " ");
            Serial.print(uidByte[i], HEX);
        }
        Serial.println();
        rfid.PICC_HaltA(); // halt PICC
        rfid.PCD_StopCrypto1(); // stop encryption on PCD
        if (isAuthorizedCard(uidByte)) {
            Serial.println("Authorized card detected.");
            unlockDoor(SOLENOID_PIN, AUTHORIZED_LED_PIN, I2C_SLAVE_ADDRESS,
                       &servo);
        } else {
            Serial.println("Unauthorized card detected.");

            digitalWrite(UNAUTHORIZED_LED_PIN, HIGH);
            tone(BUZZER_PIN, 100);
            playUnauthorizedCardMelody(BUZZER_PIN);
            noTone(BUZZER_PIN);
            digitalWrite(UNAUTHORIZED_LED_PIN, LOW);
        }
    }
}

auto sweep(Servo *servo, const int start, const int end,
           const uint8_t step,
           const uint8_t delay_ms) -> void {
    if (servo == nullptr) {
        return;
    }

    if (start < end) {
        for (int pos = start; pos <= end; pos += step) {
            servo->write(pos);
            delay(delay_ms);
        }
    } else {
        for (int pos = start; pos >= end; pos -= step) {
            servo->write(pos);
            delay(delay_ms);
        }
    }
}

auto
unlockDoor(const uint8_t solenoidPin, const uint8_t authorizedLedPin,
           const uint8_t solenoidI2cAddr, Servo *servo) -> void {
    digitalWrite(authorizedLedPin, HIGH);
    Serial.println("Door unlocked.");
    digitalWrite(solenoidPin, HIGH);
    playAuthorizedCardBeep(BUZZER_PIN);
    if (servo != nullptr) {
        sweep(servo, 0, 90);
    }
    if (solenoidI2cAddr != 0) {
        sendI2Uint8(1, solenoidI2cAddr);
    }
    delay(5000);
    if (solenoidI2cAddr != 0) {
        sendI2Uint8(0, solenoidI2cAddr);
    }

    if (servo != nullptr) {
        sweep(servo, 90, 0);
    }
    digitalWrite(solenoidPin, LOW);
    Serial.println("Door locked.");
    digitalWrite(authorizedLedPin, LOW);
}

auto playUnauthorizedCardMelody(const uint64_t buzzerPin) -> void {
    for (int i = 0; i < 3; i++) {
        tone(buzzerPin, 3000);
        delay(1000);
        noTone(buzzerPin);
        delay(1000);
    }
}

auto playAuthorizedCardBeep(const uint64_t buzzerPin) -> void {
    tone(buzzerPin, 3000, 1000);
    delay(1000);
}

auto isAuthorizedCard(const byte *uid) -> bool {
    for (const auto &authorizedCard: authorizedCards) {
        bool match = true;
        for (int j = 0; j < 4; j++) {
            if (uid[j] != authorizedCard[j]) {
                match = false;
                break;
            }
        }
        if (match) {
            return true;
        }
    }
    return false;
}