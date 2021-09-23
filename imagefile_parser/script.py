import RPi.GPIO as GPIO
import time

channel = 21

# GPIO setup
GPIO.setmode(GPIO.BCM)
GPIO.setup(channel, GPIO.OUT)


def device_on(pin):
    GPIO.output(pin, GPIO.HIGH)  # Turn device on


def device_off(pin):
    GPIO.output(pin, GPIO.LOW)  # Turn device off


if __name__ == '__main__':
    try:
        cmd = sys.argv[1]
        if cmd == "on":
            device_on(channel)
        if cmd == "off":
            device_off(channel)
        if cmd == "reboot":
            device_off(channel)
            time.sleep(1)
            device_on(channel)
        time.sleep(1)
        GPIO.cleanup()
    except KeyboardInterrupt:
        GPIO.cleanup()