FROM espressif/idf-rust:esp32c3_latest

# Default serial port for flashing
ENV ESPFLASH_PORT=/dev/ttyUSB0

WORKDIR /esp

COPY . .

ENTRYPOINT ["cargo"]
