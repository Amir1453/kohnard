EXECUTABLE = bxc.exe

all: build

build:
	cargo build --release
	cp target/release/kohnard $(EXECUTABLE)

clean:
	cargo clean
	rm -f $(EXECUTABLE)

run: build
	./$(EXECUTABLE)

.PHONY: all build clean run

