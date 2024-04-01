#include <Arduino.h>
#include <SPI.h>
#include <MFRC522.h>
#include <Wire.h>

constexpr auto SS_PIN = GPIO_NUM_5; // SDA (Slave Select) pin of RFID module
constexpr auto RST_PIN = GPIO_NUM_13; // Reset pin of RFID module
constexpr auto I2C_SDA = GPIO_NUM_26;
constexpr auto I2C_SCL = GPIO_NUM_25;
constexpr auto I2C_SLAVE_ADDRESS = 0x08;
const auto AUTHORIZED_LED_PIN = GPIO_NUM_17;
const auto UNAUTHORIZED_LED_PIN = GPIO_NUM_16;
const auto BUZZER_PIN = GPIO_NUM_27;

MFRC522 rfid(SS_PIN, RST_PIN); // Create RFID instance

// List of authorized RFID card UIDs
byte authorizedCards[][4] = {
    {0xE3, 0x95, 0x0C, 0x0E}, // Card UID 1 {0x9A, 0xBC, 0xDE, 0xF0} // Card UID 2
    {0x13, 0x85, 0x07, 0x15}, // Card UID 1 {0x9A, 0xBC, 0xDE, 0xF0} // Card UID 2
};

auto isAuthorizedCard(const byte *uid) -> bool;

auto sendI2CChar(uint8_t address, char c) -> void;

auto unlockDoor(uint64_t i2cSlaveAddress) -> void;

auto playUnauthorizedCardMelody(uint64_t buzzerPin) -> void;

auto playAuthorizedCardBeep(uint64_t buzzerPin) -> void;

auto setup() -> void {
    Serial.begin(9600);
    SPI.begin();
    rfid.PCD_Init(); // Initialize RFID reader
    Wire.begin(I2C_SDA, I2C_SCL);
    pinMode(AUTHORIZED_LED_PIN, OUTPUT);
    pinMode(UNAUTHORIZED_LED_PIN, OUTPUT);
    pinMode(BUZZER_PIN, OUTPUT);
    sendI2CChar(I2C_SLAVE_ADDRESS, '0');
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
            unlockDoor(I2C_SLAVE_ADDRESS);
        } else {
            Serial.println("Unauthorized card detected.");

            digitalWrite(UNAUTHORIZED_LED_PIN, HIGH);
            tone(BUZZER_PIN, 100);
            playUnauthorizedCardMelody(BUZZER_PIN);
            noTone(BUZZER_PIN);
            digitalWrite(UNAUTHORIZED_LED_PIN, LOW);
            sendI2CChar(I2C_SLAVE_ADDRESS, '0');
        }
    }
}


auto unlockDoor(const uint64_t i2cSlaveAddress) -> void {
    digitalWrite(AUTHORIZED_LED_PIN, HIGH);
    Serial.println("Door unlocked.");
    playAuthorizedCardBeep(BUZZER_PIN);
    sendI2CChar(i2cSlaveAddress, '1');
    delay(3000);
    sendI2CChar(i2cSlaveAddress, '0');
    Serial.println("Door locked.");
    digitalWrite(AUTHORIZED_LED_PIN, LOW);
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
    tone(buzzerPin, 3000);
    delay(1000);
    noTone(buzzerPin);
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

auto sendI2CChar(const uint8_t address,
                 char c) -> void {
    Wire.beginTransmission(address);
    Wire.write(c);
    Wire.endTransmission();
}
