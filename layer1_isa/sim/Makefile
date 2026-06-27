#==============================================================================
# Makefile — TPT ISA Simulation Build
#==============================================================================
# TPT GPU — Tensor Processing Technology
# License: Apache License 2.0 (with Express Patent Grant)
#==============================================================================
#
# Tools required:
#   - Icarus Verilog (iverilog) for simulation
#   - Python 3.x for assembler
#   - gtkwave (optional) for waveform viewing
#
# Targets:
#   make sim      — Compile and run the simulation
#   make asm      — Assemble the test program
#   make wave     — View waveforms with gtkwave
#   make clean    — Remove build artifacts
#==============================================================================

# Tools
IVL = iverilog
VVP = vvp
PY  = python3
GTKWAVE = gtkwave

# Directories
RTL_DIR = ../rtl
SIM_DIR = .
PROG_DIR = programs

# Source files
RTL_SRC = \
    $(RTL_DIR)/tpt_pkg.sv \
    $(RTL_DIR)/tpt_decode.sv \
    $(RTL_DIR)/tpt_regfile.sv \
    $(RTL_DIR)/tpt_alu.sv \
    $(RTL_DIR)/tpt_lsu.sv \
    $(RTL_DIR)/tpt_ctrl.sv \
    $(RTL_DIR)/tpt_tensor_unit.sv \
    $(RTL_DIR)/tpt_pipeline.sv \
    $(RTL_DIR)/tpt_core.sv

TB_SRC = $(SIM_DIR)/tpt_tb.sv

# Outputs
VVP_OUT = tpt_sim.vvp
VCD_OUT = tpt_sim.vcd

#------------------------------------------------------------------------------
# Default target
#------------------------------------------------------------------------------
all: sim

#------------------------------------------------------------------------------
# Assemble the test program
#------------------------------------------------------------------------------
asm:
	$(PY) tpt_assemble.py $(PROG_DIR)/simple_add.asm

#------------------------------------------------------------------------------
# Compile and run simulation
#------------------------------------------------------------------------------
sim: asm
	$(IVL) -g2012 -o $(VVP_OUT) $(RTL_SRC) $(TB_SRC)
	$(VVP) $(VVP_OUT)

#------------------------------------------------------------------------------
# View waveforms
#------------------------------------------------------------------------------
wave: $(VCD_OUT)
	$(GTKWAVE) $(VCD_OUT)

#------------------------------------------------------------------------------
# Clean build artifacts
#------------------------------------------------------------------------------
clean:
	rm -f $(VVP_OUT) $(VCD_OUT)
	rm -rf work/
