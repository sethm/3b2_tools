CC=g++
CFLAGS=-g -c -Wall -std=c++11
LDFLAGS=
SOURCES=imgread.cc
OBJECTS=$(SOURCES:.cc=.o)
EXECUTABLE=imgread

all: $(SOURCES) $(EXECUTABLE)

clean:
	rm -f $(EXECUTABLE) *.o
    
$(EXECUTABLE): $(OBJECTS) 
	$(CC) $(LDFLAGS) $(OBJECTS) -o $@

.cc.o:
	$(CC) $(CFLAGS) $< -o $@
