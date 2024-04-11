// Include the necessary libraries
#include <Arduino.h>
#include <ESP32Servo.h>

class ColorSensor {
private:
    uint8_t s0;
    uint8_t s1;
    uint8_t s2;
    uint8_t s3;
    uint8_t out;

    uint8_t sensitivity;

    void setSensitivity(uint8_t value) {
        sensitivity = value;
    }

public:
    ColorSensor(uint8_t s0, uint8_t s1, uint8_t s2, uint8_t s3, uint8_t out,
                uint8_t sensitivity = 5) :
        s0{s0}, s1{s1}, s2{s2}, s3{s3}, out{out}, sensitivity{sensitivity} {

        // Set the TCS3200 color sensor pins
        pinMode(s0, OUTPUT);
        pinMode(s1, OUTPUT);
        pinMode(s2, OUTPUT);
        pinMode(s3, OUTPUT);
        pinMode(out, INPUT);

        // Configure the TCS3200 color sensor
        digitalWrite(s0, HIGH);
        digitalWrite(s1, HIGH);
    }

    void read(uint16_t &r, uint16_t &g, uint16_t &b) const {
        // Configure the TCS3200 color sensor
        digitalWrite(s2, LOW);
        digitalWrite(s3, LOW);

        // Read the red value
        r = pulseIn(out, digitalRead(out) == LOW ? HIGH : LOW);

        // Read the green value
        digitalWrite(s2, HIGH);
        digitalWrite(s3, HIGH);
        g = pulseIn(out, digitalRead(out) == LOW ? HIGH : LOW);

        // Read the blue value
        digitalWrite(s2, LOW);
        digitalWrite(s3, HIGH);
        b = pulseIn(out, digitalRead(out) == LOW ? HIGH : LOW);
    }
};


// Define the servo pins
constexpr int PAPER_SERVO_PIN = 2;
constexpr int PLASTIC_SERVO_PIN = 3;

// Define the ultrasonic sensor pins
constexpr int PAPER_TRIGGER_PIN = 4;
constexpr int PAPER_ECHO_PIN = 5;
constexpr int PLASTIC_TRIGGER_PIN = 6;
constexpr int PLASTIC_ECHO_PIN = 7;

// Define the TCS3200 color sensor pins
constexpr int CS1_S0_PIN = 9;
constexpr int CS1_S1_PIN = 10;
constexpr int CS1_S2_PIN = 11;
constexpr int CS1_S3_PIN = 12;
constexpr int CS1_OUT_PIN = 13;
//constexpr int CS2_S0_PIN = 9;
//constexpr int CS2_S1_PIN = 10;
//constexpr int CS2_S2_PIN = 11;
//constexpr int CS2_S3_PIN = 12;
//constexpr int CS2_OUT_PIN = 13;

// Defines the sensitivity of the color sensor
constexpr uint8_t CS1_COLOR_SENSITIVITY = 5;
//constexpr uint8_t CS2_COLOR_SENSITIVITY = 5;

// Create servo objects
Servo paperServo;
Servo plasticServo;

// Create color sensor objects
ColorSensor cs1(CS1_S0_PIN, CS1_S1_PIN, CS1_S2_PIN, CS1_S3_PIN, CS1_OUT_PIN,
                CS1_COLOR_SENSITIVITY);
//ColorSensor cs2(CS2_S0_PIN, CS2_S1_PIN, CS2_S2_PIN, CS2_S3_PIN, CS2_OUT_PIN,
//                CS2_COLOR_SENSITIVITY);

// Define the maximum distance for considering the trash can as full
constexpr int MAX_DISTANCE = 10; // in cm


// Define the colors for paper and plastic
constexpr int16_t PAPER_COLORS[][3] = {
    {36, 36, 30},
    {23, 26, 24}
};

constexpr int16_t PLASTIC_COLORS[][3] = {
    {8,  8,  7},
    {10, 23, 20}
};

// Forward declarations
void openLid(Servo &servo);

uint64_t getDistance(int triggerPin, int echoPin);

bool isColorPaper(uint16_t r, uint16_t g, uint16_t b);

bool isColorPlastic(uint16_t r, uint16_t g, uint16_t b);

void setup() {
    // Initialize serial communication
    Serial.begin(9600);

    // Attach the servos to their respective pins
    paperServo.attach(PAPER_SERVO_PIN);
    plasticServo.attach(PLASTIC_SERVO_PIN);

    // Set the servos to their initial positions
    paperServo.write(0);
    plasticServo.write(0);

    // Set the ultrasonic sensor pins
    pinMode(PAPER_TRIGGER_PIN, OUTPUT);
    pinMode(PAPER_ECHO_PIN, INPUT);
    pinMode(PLASTIC_TRIGGER_PIN, OUTPUT);
    pinMode(PLASTIC_ECHO_PIN, INPUT);
}

void loop() {
    // Read the color values from the TCS3200 sensor
    uint16_t red, green, blue;
    cs1.read(red, green, blue);

    const auto logRgbValues = [=] {
        Serial.print("RGB Values: { ");
        Serial.print(red);
        Serial.print(", ");
        Serial.print(green);
        Serial.print(", ");
        Serial.print(blue);
        Serial.println(" }, ");
    };

    const auto isPaper = isColorPaper(red, green, blue);
    const auto isPlastic = isColorPlastic(red, green, blue);

    if (isPaper || isPlastic) {
        if (isPaper) {
            Serial.print("Paper detected: ");
            logRgbValues();

            // Check if the paper trash can is not full
            if (getDistance(PAPER_TRIGGER_PIN, PAPER_ECHO_PIN) > MAX_DISTANCE) {
                openLid(paperServo); // Open the lid for paper
            } else {
                Serial.println("Paper trash can is full");
            }
        }

        if (isPlastic) {
            Serial.print("Plastic detected: ");
            logRgbValues();

            // Check if the plastic trash can is not full
            if (getDistance(PLASTIC_TRIGGER_PIN, PLASTIC_ECHO_PIN) >
                MAX_DISTANCE) {
                openLid(plasticServo); // Open the lid for plastic
            } else {
                Serial.println("Plastic trash can is full");
            }
        }
        // Check if the detected color matches known paper or plastic colors
    } else {
        // Unrecognized color
        Serial.print("Unrecognized color: ");
        logRgbValues();
    }

    delay(1000);
}

void openLid(Servo &servo) {
    servo.write(90);
    delay(5000); // Wait for 3 seconds
    servo.write(0);
}

uint64_t getDistance(int triggerPin, int echoPin) {
    // Clear the trigger pin
    digitalWrite(triggerPin, LOW);
    delayMicroseconds(2);

    // Set the trigger pin to HIGH for 10 microseconds
    digitalWrite(triggerPin, HIGH);
    delayMicroseconds(10);
    digitalWrite(triggerPin, LOW);

    // Read the echo pin and calculate the distance
    uint64_t duration = pulseIn(echoPin, HIGH);
    uint64_t distance = duration * 0.034 / 2; // Speed of sound: 340 m/s

    return distance;
}

bool isColorPaper(uint16_t r, uint16_t g, uint16_t b) {
    for (const auto &color: PAPER_COLORS) {
        if (r >= color[0] - CS1_COLOR_SENSITIVITY &&
            r <= color[0] + CS1_COLOR_SENSITIVITY &&
            g >= color[1] - CS1_COLOR_SENSITIVITY &&
            g <= color[1] + CS1_COLOR_SENSITIVITY &&
            b >= color[2] - CS1_COLOR_SENSITIVITY &&
            b <= color[2] + CS1_COLOR_SENSITIVITY) {
            return true;
        }
    }
    return false;
}

bool isColorPlastic(uint16_t r, uint16_t g, uint16_t b) {
    for (const auto &color: PLASTIC_COLORS) {
        if (r >= color[0] - CS1_COLOR_SENSITIVITY &&
            r <= color[0] + CS1_COLOR_SENSITIVITY &&
            g >= color[1] - CS1_COLOR_SENSITIVITY &&
            g <= color[1] + CS1_COLOR_SENSITIVITY &&
            b >= color[2] - CS1_COLOR_SENSITIVITY &&
            b <= color[2] + CS1_COLOR_SENSITIVITY) {
            return true;
        }
    }
    return false;
}