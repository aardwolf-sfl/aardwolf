CC = gcc
CFLAGS = -g -O2 -Wall -Wextra
LDFLAGS =

SRCDIR = src
OUTDIR = bin

EXECUTABLE = sorted
SOURCES = $(wildcard $(SRCDIR)/*.c)
OBJECTS = $(patsubst $(SRCDIR)/%.c, $(OUTDIR)/%.o, $(SOURCES))

all: build $(EXECUTABLE)

$(EXECUTABLE): $(OBJECTS)
	$(CC) $(CFLAGS) -o $(OUTDIR)/$@ $(OBJECTS) $(LDFLAGS)

$(OBJECTS): $(OUTDIR)/%.o : $(SRCDIR)/%.c
	$(CC) $(CFLAGS) -o $@ -c $<

build:
	@mkdir -p $(OUTDIR)

test:
	@cd tests ; bash run.sh

clean:
	rm -rf $(OUTDIR)
