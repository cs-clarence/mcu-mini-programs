/// include library

#include <Arduino.h>
#include <Servo.h> ///servo motor library


// Pin Definitions


//plastic bin pin definitions
#define redpin 5
#define bluepin 6
#define greenpin 7
#define plasticsensor A0
#define metalsensor A1
#define IR_plastic A3

//metal bin pin definitions
#define redpin_M 8
#define bluepin_M 9
#define greenpin_M 10
#define metalsensor_M A2
#define IR_metal A3


Servo myservo; // create servo object to control a servo on plastic bin
Servo myservo_M; // create servo object to control a servo on metal bin

const int MAX_SWEEP = 60;
const int MIN_SWEEP = 180;

void setup() {
  myservo.attach(11);
  myservo_M.attach(12);

  myservo.write(MIN_SWEEP);
  myservo_M.write(MIN_SWEEP);

  // attaches the plastic bin servo on pin 11 to the servo object
  // attaches the metal bin servo on pin 12 to the servo object
  //RGB LED on plastic bin
  pinMode(redpin,OUTPUT);
  pinMode(bluepin,OUTPUT);
  pinMode(greenpin,OUTPUT);
  //RGB LED on metal bin
  pinMode(redpin_M,OUTPUT);
  pinMode(bluepin_M,OUTPUT);
  pinMode(greenpin_M,OUTPUT);
  //Create sensor values in pull up condition
  pinMode(plasticsensor,INPUT_PULLUP);
  pinMode(metalsensor,INPUT_PULLUP);
  pinMode(metalsensor_M,INPUT_PULLUP);
  //Activate IR sensor
  pinMode(IR_plastic,INPUT);
  pinMode(IR_metal,INPUT);
  //start serial monitor
  Serial.begin(9600);
  //Set RGB LED on deafult vaues (white)
  analogWrite(redpin, 255);
  analogWrite(bluepin, 255);
  analogWrite(greenpin, 255);
  analogWrite(redpin_M, 255);
  analogWrite(bluepin_M, 255);
  analogWrite(greenpin_M, 255);
}

void sweep(Servo &servo, int start, int end, uint8_t step = 1,
           uint8_t delay_ms = 1) {
  if (start < end) {
    for (int pos = start; pos <= end; pos += step) {
      servo.write(pos);
      delay(delay_ms);
    }
  } else {
    for (int pos = start; pos >= end; pos -= step) {
      servo.write(pos);
      delay(delay_ms);
    }
  }
}


void loop() {
  int sensor_read_plastic = digitalRead(plasticsensor);
  int sensor_read_metal = digitalRead(metalsensor);
  int sensor_read_metaletal2 = digitalRead(metalsensor_M);
  int sensor_read_IR_plastic = digitalRead(IR_plastic);
  int sensor_read_IR_metal = digitalRead(IR_metal);
  Serial.println("plastic sensor");
  Serial.println(sensor_read_plastic);
  Serial.println(sensor_read_metal);
  Serial.println("metal sensor");
  Serial.println(sensor_read_metaletal2);

  ///Check if the trash bin is full

  //plastic bin
  if (sensor_read_IR_plastic == 1) {
    analogWrite(redpin, 255);
    analogWrite(bluepin, 0);
    analogWrite(greenpin, 0);
  }
  //metal bin
  if (sensor_read_IR_metal == 1) {
    analogWrite(redpin_M, 255);
    analogWrite(bluepin_M, 0);
    analogWrite(greenpin_M, 0);
  }

  //Plastic bin
  if (sensor_read_plastic == 1 && sensor_read_metal == 0) {
    sweep(
      myservo,
      MIN_SWEEP,
      MAX_SWEEP
    );
    delay(2500);
    // analogWrite(redpin, 255);
    // analogWrite(bluepin, 0);
    // analogWrite(greenpin, 255);
    sweep(
      myservo,
      MAX_SWEEP,
      MIN_SWEEP
    );


    myservo.write(MAX_SWEEP);
    // analogWrite(redpin, 0);
    // analogWrite(bluepin, 0);
    // analogWrite(greenpin, 0);
  } else {
    // keep the door close
    myservo.write(MIN_SWEEP);
    // analogWrite(redpin, 255);
    // analogWrite(bluepin, 255);
    // analogWrite(greenpin, 255);
  }


  //metal bin


  if (sensor_read_metaletal2 == 1) {
    sweep(
      myservo_M,
      MIN_SWEEP,
      MAX_SWEEP
    );
    // analogWrite(redpin_M, 255);
    // analogWrite(bluepin_M, 0);
    // analogWrite(greenpin_M, 255);
    delay(2500);
    sweep(
      myservo_M,
      MAX_SWEEP,
      MIN_SWEEP
    );
    // analogWrite(redpin_M, 0);
    // analogWrite(bluepin_M, 0);
    // analogWrite(greenpin_M, 0);
  } else {
    // keep the door close
    myservo_M.write(MIN_SWEEP);


    // analogWrite(redpin_M, 255);
    // analogWrite(bluepin_M, 255);
    // analogWrite(greenpin_M, 255);
  }
}
