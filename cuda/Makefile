objects = build/batch_inverse.o build/bit_reverse.o build/circle.o build/fields.o build/fri.o build/utils.o

all: $(objects)
	nvcc -Xcompiler -fPIC -shared -arch=sm_50 $(objects) -o libgpubackend.so

build/%.o: src/%.cu
	mkdir -p build/
	nvcc -Xcompiler -fPIC -x cu -arch=sm_50 -I. -dc $< -o $@

clean:
	rm -f build/*.o libgpubackend.so