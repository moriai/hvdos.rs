VPATH = $(SRCDIR)
SRCDIR = src
SRCFILES = DOSKernel.cpp hvdos.cpp
INCFILES = DOSKernel.h interface.h vmcs.h
CXX = clang++ -Wall -Wextra -Wno-unused-parameter

all: hvdos

hvdos: $(SRCFILES)
	$(CXX) -std=c++17 -O3 -framework Hypervisor -o $@ -I$(SRCDIR) $^

$(SRCFILES): $(INCFILES)

clean:
	rm -f hvdos
