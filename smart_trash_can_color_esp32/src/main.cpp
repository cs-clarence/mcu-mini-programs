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
constexpr int PAPER_TRIGGER_PIN = GPIO_NUM_14;
constexpr int PAPER_ECHO_PIN = GPIO_NUM_12;
constexpr int PLASTIC_TRIGGER_PIN = GPIO_NUM_4;
constexpr int PLASTIC_ECHO_PIN = GPIO_NUM_2;
constexpr bool ULTRASONIC_ENABLED = true;

// Define the servo pins
constexpr int PAPER_SERVO_PIN = GPIO_NUM_13;
constexpr int PLASTIC_SERVO_PIN = GPIO_NUM_15;

// Define the TCS3200 color sensor pins
constexpr int CS1_S0_PIN = GPIO_NUM_32;
constexpr int CS1_S1_PIN = GPIO_NUM_33;
constexpr int CS1_S2_PIN = GPIO_NUM_25;
constexpr int CS1_S3_PIN = GPIO_NUM_26;
constexpr int CS1_OUT_PIN = GPIO_NUM_27;
constexpr int CS2_S0_PIN = GPIO_NUM_19;
constexpr int CS2_S1_PIN = GPIO_NUM_18;
constexpr int CS2_S2_PIN = GPIO_NUM_5;
constexpr int CS2_S3_PIN = GPIO_NUM_17;
constexpr int CS2_OUT_PIN = GPIO_NUM_16;

// Defines the sensitivity of the color sensor
constexpr uint8_t CS1_SENSITIVITY = 3;
constexpr uint8_t CS2_SENSITIVITY = 3;

// Create servo objects
Servo paperServo;
Servo plasticServo;

// Create color sensor objects
ColorSensor cs1(CS1_S0_PIN, CS1_S1_PIN, CS1_S2_PIN, CS1_S3_PIN, CS1_OUT_PIN);
ColorSensor cs2(CS2_S0_PIN, CS2_S1_PIN, CS2_S2_PIN, CS2_S3_PIN, CS2_OUT_PIN);

// Define the maximum distance for considering the trash can as full
constexpr int MAX_DISTANCE = 10; // in cm


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

constexpr int16_t PLASTIC_COLORS[][3] = {
    {36, 36, 30},
    {44, 29, 37},
    {29, 15, 24},
    {3,  4,  10},
    {14, 13, 10},
};

// Forward declarations
void openLid(Servo &servo);

uint64_t getDistance(int triggerPin, int echoPin);

bool isColorPaper(uint16_t r, uint16_t g, uint16_t b, uint8_t sensitivity = 5);

bool
isColorPlastic(uint16_t r, uint16_t g, uint16_t b, uint8_t sensitivity = 5);

void setup() {
    // Initialize serial communication
    Serial.begin(115200);

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

    const auto logRgbValues = [&] {
        Serial.print("RGB Values: { ");
        Serial.print(red);
        Serial.print(", ");
        Serial.print(green);
        Serial.print(", ");
        Serial.print(blue);
        Serial.println(" }, ");
    };


    if (isColorPaper(red, green, blue, CS1_SENSITIVITY)) {
        Serial.print("Paper detected: ");
        logRgbValues();

        // Check if the paper trash can is not full
        if ((getDistance(PAPER_TRIGGER_PIN, PAPER_ECHO_PIN) > MAX_DISTANCE) ||
            !ULTRASONIC_ENABLED) {
            openLid(paperServo); // Open the lid for paper
        } else {
            Serial.println("Paper trash can is full");
        }
    } else {
        Serial.print("Unknown paper color detected: ");
        logRgbValues();
    }

    cs2.read(red, green, blue);

    if (isColorPlastic(red, green, blue, CS2_SENSITIVITY)) {
        Serial.print("Plastic detected: ");
        logRgbValues();

        // Check if the plastic trash can is not full
        if ((getDistance(PLASTIC_TRIGGER_PIN, PLASTIC_ECHO_PIN) >
             MAX_DISTANCE) || !ULTRASONIC_ENABLED) {
            openLid(plasticServo); // Open the lid for plastic
        } else {
            Serial.println("Plastic trash can is full");
        }
    } else {
        Serial.print("Unknown plastic color detected: ");
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

bool
isColorPlastic(uint16_t r, uint16_t g, uint16_t b, uint8_t sensitivity) {
    for (const auto &color: PLASTIC_COLORS) {
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