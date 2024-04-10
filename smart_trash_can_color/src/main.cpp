// Include the necessary libraries
#include <Arduino.h>
#include <Servo.h>

// Define the servo pins
constexpr int PAPER_SERVO_PIN = 2;
constexpr int PLASTIC_SERVO_PIN = 3;

// Define the ultrasonic sensor pins
constexpr int PAPER_TRIGGER_PIN = 4;
constexpr int PAPER_ECHO_PIN = 5;
constexpr int PLASTIC_TRIGGER_PIN = 6;
constexpr int PLASTIC_ECHO_PIN = 7;

// Define the TCS3200 color sensor pins
constexpr int S0_PIN = 9;
constexpr int S1_PIN = 10;
constexpr int S2_PIN = 11;
constexpr int S3_PIN = 12;
constexpr int OUT_PIN = 13;


// Create servo objects
Servo paperServo;
Servo plasticServo;

// Define the maximum distance for considering the trash can as full
constexpr int MAX_DISTANCE = 10; // in cm

// Defines the sensitivity of the color sensor
constexpr uint8_t COLOR_SENSITIVITY = 5;

// Define the colors for paper and plastic
constexpr int32_t PAPER_COLORS[][3] = {
    {36, 36, 30},
    {23, 26, 24}
};

constexpr int32_t PLASTIC_COLORS[][3] = {
    {8, 8, 7},
    {10, 23, 20}
};

// Forward declarations
void openLid(Servo &servo);

uint64_t getDistance(int triggerPin, int echoPin);

void readColor(uint16_t &r, uint16_t &g, uint16_t &b);

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

    // Set the TCS3200 color sensor pins
    pinMode(S0_PIN, OUTPUT);
    pinMode(S1_PIN, OUTPUT);
    pinMode(S2_PIN, OUTPUT);
    pinMode(S3_PIN, OUTPUT);
    pinMode(OUT_PIN, INPUT);

    // Configure the TCS3200 color sensor
    digitalWrite(S0_PIN, HIGH);
    digitalWrite(S1_PIN, HIGH);

}

void loop() {
    // Read the color values from the TCS3200 sensor
    uint16_t red, green, blue;
    readColor(red, green, blue);

    const auto logRgbValues = [red, green, blue] {
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

void readColor(uint16_t &r, uint16_t &g, uint16_t &b) {
    // Configure the TCS3200 color sensor
    digitalWrite(S2_PIN, LOW);
    digitalWrite(S3_PIN, LOW);

    // Read the red value
    r = pulseIn(OUT_PIN, digitalRead(OUT_PIN) == LOW ? HIGH : LOW);

    // Read the green value
    digitalWrite(S2_PIN, HIGH);
    digitalWrite(S3_PIN, HIGH);
    g = pulseIn(OUT_PIN, digitalRead(OUT_PIN) == LOW ? HIGH : LOW);

    // Read the blue value
    digitalWrite(S2_PIN, LOW);
    digitalWrite(S3_PIN, HIGH);
    b = pulseIn(OUT_PIN, digitalRead(OUT_PIN) == LOW ? HIGH : LOW);
}

bool isColorPaper(uint16_t r, uint16_t g, uint16_t b) {
    for (const auto &color: PAPER_COLORS) {
        if (r >= color[0] - COLOR_SENSITIVITY &&
            r <= color[0] + COLOR_SENSITIVITY &&
            g >= color[1] - COLOR_SENSITIVITY &&
            g <= color[1] + COLOR_SENSITIVITY &&
            b >= color[2] - COLOR_SENSITIVITY &&
            b <= color[2] + COLOR_SENSITIVITY) {
            return true;
        }
    }
    return false;
}

bool isColorPlastic(uint16_t r, uint16_t g, uint16_t b) {
    for (const auto &color: PLASTIC_COLORS) {
        if (r >= color[0] - COLOR_SENSITIVITY &&
            r <= color[0] + COLOR_SENSITIVITY &&
            g >= color[1] - COLOR_SENSITIVITY &&
            g <= color[1] + COLOR_SENSITIVITY &&
            b >= color[2] - COLOR_SENSITIVITY &&
            b <= color[2] + COLOR_SENSITIVITY) {
            return true;
        }
    }
    return false;
}