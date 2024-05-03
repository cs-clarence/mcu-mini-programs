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
constexpr auto BUZZER_CHANNEL = 2;
constexpr auto BUZZER_FREQ = 2000;
constexpr auto BUZZER_RESOLUTION = 8;
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
                uint8_t solenoidI2cAddr,
                Servo *servo,
                uint8_t buzzerPin,
                uint8_t buzzerChannel
) -> void;

auto lockDoor(uint8_t solenoidPin, uint8_t authorizedLedPin,
                uint8_t solenoidI2cAddr,
                Servo *servo,
                uint8_t buzzerPin,
                uint8_t buzzerChannel
) -> void;

auto playUnauthorizedCardMelody(uint8_t pin, uint8_t channel) -> void;

auto playAuthorizedCardBeep(uint8_t pin, uint8_t channel) -> void;

auto findAuthorizedCardIndex(const byte *uid, std::size_t &index) -> bool;

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
    ledcSetup(BUZZER_CHANNEL, BUZZER_FREQ, BUZZER_RESOLUTION);
}

bool isOpen = false;
std::size_t lastCardIndex = 0;

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

            if (isOpen) {
                lockDoor(SOLENOID_PIN,
                           AUTHORIZED_LED_PIN,
                           I2C_SLAVE_ADDRESS,
                           &servo,
                           BUZZER_PIN,
                           BUZZER_CHANNEL
                );
                isOpen = false;
            } else {
                if (findAuthorizedCardIndex(uidByte, lastCardIndex)) {
                    Serial.println("Authorized card detected.");
                }

                unlockDoor(SOLENOID_PIN,
                           AUTHORIZED_LED_PIN,
                           I2C_SLAVE_ADDRESS,
                           &servo,
                           BUZZER_PIN,
                           BUZZER_CHANNEL
                );
                isOpen = true;
            }

        } else {
            Serial.println("Unauthorized card detected.");
            digitalWrite(UNAUTHORIZED_LED_PIN, HIGH);
            playUnauthorizedCardMelody(BUZZER_PIN, BUZZER_CHANNEL);
            digitalWrite(UNAUTHORIZED_LED_PIN, LOW);
        }
    }
}

auto
unlockDoor(const uint8_t solenoidPin, const uint8_t authorizedLedPin,
           const uint8_t solenoidI2cAddr, Servo *servo,
           const uint8_t buzzerPin,
           const uint8_t buzzerChannel
) -> void {
    digitalWrite(authorizedLedPin, HIGH);
    Serial.println("Door unlocked.");
    digitalWrite(solenoidPin, HIGH);
    playAuthorizedCardBeep(buzzerPin, buzzerChannel);
    if (servo != nullptr) {
        servo->write(90);
        delay(1000);
    }
    if (solenoidI2cAddr != 0) {
        sendI2Uint8(1, solenoidI2cAddr);
    }
}

auto
lockDoor(const uint8_t solenoidPin, const uint8_t authorizedLedPin,
         const uint8_t solenoidI2cAddr, Servo *servo,
         const uint8_t buzzerPin,
         const uint8_t buzzerChannel) -> void {
    playAuthorizedCardBeep(buzzerPin, buzzerChannel);
    if (solenoidI2cAddr != 0) {
        sendI2Uint8(0, solenoidI2cAddr);
    }

    if (servo != nullptr) {
        servo->write(0);
        delay(1000);
    }
    digitalWrite(solenoidPin, LOW);
    Serial.println("Door locked.");
    digitalWrite(authorizedLedPin, LOW);
}


auto
playUnauthorizedCardMelody(const uint8_t pin, const uint8_t channel) -> void {
    ledcAttachPin(pin, channel);
    for (int i = 0; i < 3; i++) {
        ledcWrite(channel, 64);
        delay(500);
        ledcWrite(channel, 0);
        delay(500);
    }
    ledcWrite(channel, 0);
    ledcDetachPin(pin);
}

auto
playAuthorizedCardBeep(const uint8_t pin, const uint8_t channel) -> void {
    ledcAttachPin(pin, channel);
    ledcWrite(channel, 96);
    delay(1000);
    ledcWrite(channel, 0);
    delay(1000);
    ledcDetachPin(pin);
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

auto findAuthorizedCardIndex(const byte *uid, std::size_t &index) -> bool {
    constexpr std::size_t AUTHORIZED_SIZED_CARDS =
        sizeof(authorizedCards) / sizeof(authorizedCards[0]);

    for (std::size_t i = 0; i < AUTHORIZED_SIZED_CARDS; i++) {
        bool match = true;

        for (int j = 0; j < 4; j++) {
            if (uid[j] != authorizedCards[i][j]) {
                match = false;
                break;
            }
        }

        if (match) {
            index = i;
            return true;
        }
    }

    return false;
}