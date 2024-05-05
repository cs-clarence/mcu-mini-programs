// Include the necessary libraries
#include <Arduino.h>
#include <Servo.h>

class ColorSensor {
private:
    uint8_t s0;
    uint8_t s1;
    uint8_t s2;
    uint8_t s3;
    uint8_t out;


public:
    ColorSensor(uint8_t s0, uint8_t s1, uint8_t s2, uint8_t s3, uint8_t out) :
        s0{s0}, s1{s1}, s2{s2}, s3{s3}, out{out} {
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

// Define the ultrasonic sensor pins
constexpr int PAPER_TRIGGER_PIN = 7;
constexpr int PAPER_ECHO_PIN = 8;
constexpr bool ULTRASONIC_ENABLED = false;

// Define the servo pins
constexpr int PAPER_SERVO_PIN = 9;

// Define the TCS3200 color sensor pins
constexpr int PAPER_CS_S0_PIN = 2;
constexpr int PAPER_CS_S1_PIN = 3;
constexpr int PAPER_CS_S2_PIN = 4;
constexpr int PAPER_CS_S3_PIN = 5;
constexpr int PAPER_CS_OUT_PIN = 6;

// Defines the sensitivity of the color sensor
constexpr uint8_t PAPER_CS_SENSITIVITY = 3;

// Create servo objects
Servo paperServo;
Servo plasticServo;

// Create color sensor objects
ColorSensor paperCs(PAPER_CS_S0_PIN, PAPER_CS_S1_PIN, PAPER_CS_S2_PIN,
                    PAPER_CS_S3_PIN, PAPER_CS_OUT_PIN);

// Define the maximum distance for considering the trash can as full
constexpr int MIN_DISTANCE = 10; // in cm


// Define the colors for paper and plastic
constexpr int16_t PAPER_COLORS[][3] = {
    {8,  8,  7},
    {14, 19, 19},
    {23, 29, 28},
    {18, 19, 21},
    {17, 20, 22},
    {37, 35, 36},
    {30, 36, 34},
    {19, 24, 23},
};

// Forward declarations
void openLid(Servo &servo);

uint64_t getDistance(int triggerPin, int echoPin);

bool isColorPaper(uint16_t r, uint16_t g, uint16_t b, uint8_t sensitivity = 5);

void setup() {
    // Initialize serial communication
    Serial.begin(115200);

    // Attach the servos to their respective pins
    paperServo.attach(PAPER_SERVO_PIN);

    // Set the servos to their initial positions
    paperServo.write(0);
    plasticServo.write(0);

    // Set the ultrasonic sensor pins
    pinMode(PAPER_TRIGGER_PIN, OUTPUT);
    pinMode(PAPER_ECHO_PIN, INPUT);
}

void loop() {
    // Read the color values from the TCS3200 sensor
    uint16_t red, green, blue;
    paperCs.read(red, green, blue);

    const auto logRgbValues = [&] {
        Serial.print("RGB Values: { ");
        Serial.print(red);
        Serial.print(", ");
        Serial.print(green);
        Serial.print(", ");
        Serial.print(blue);
        Serial.println(" }, ");
    };


    if (isColorPaper(red, green, blue, PAPER_CS_SENSITIVITY)) {
        Serial.print("Paper detected: ");
        logRgbValues();

        auto distance = getDistance(PAPER_TRIGGER_PIN, PAPER_ECHO_PIN);
        bool paperFull = distance <= MIN_DISTANCE;
        // Check if the paper trash can is not full
        if (paperFull) {
            Serial.println("Paper trash can is full");
        } else {
            Serial.println("Paper trash can is not full");
        }

        if (!paperFull || !ULTRASONIC_ENABLED) {
            Serial.println("Opening paper lid");
            openLid(paperServo); // Open the lid for paper
        }
    } else {
        Serial.print("Unknown paper color detected: ");
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

bool isColorPaper(uint16_t r, uint16_t g, uint16_t b, uint8_t sensitivity) {
    for (const auto &color: PAPER_COLORS) {
        if (r >= color[0] - sensitivity &&
            r <= color[0] + sensitivity &&
            g >= color[1] - sensitivity &&
            g <= color[1] + sensitivity &&
            b >= color[2] - sensitivity &&
            b <= color[2] + sensitivity) {
            return true;
        }
    }
    return false;
}