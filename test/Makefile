CC:=gcc
AR:=ar
PRG:=test
LIB:=libtest.a
CFLAGS:=
LDFLAGS:=-L./

all: $(PRG)

$(LIB): foo.o bar.o
	$(AR) rcs $@ foo.o bar.o

$(PRG): $(LIB) main.o
	$(CC) $(LDFLAGS) main.o -o $@ -ltest

%.o: %.c Makefile
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -fr *.o
	rm -fr *.a
	rm -fr $(PRG)

