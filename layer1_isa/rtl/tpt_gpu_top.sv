//==============================================================================
// tpt_gpu_top.sv — TPT GPU Top-Level Integration
//==============================================================================
// TPT GPU — Tensor Processing Technology
// License: Apache License 2.0 (with Express Patent Grant)
//
// Top-level module integrating the full TPT GPU subsystem for silicon.
// Host Interface: PCIe-like bus with MMIO + DMA.
//==============================================================================

module tpt_gpu_top #(
    parameter int NUM_SM = 1
) (
    input  logic              clk_i,
    input  logic              rst_n_i,

    // Host MMIO interface (PCIe BAR0)
    input  logic [11:0]       host_mmio_addr_i,
    input  logic              host_mmio_req_i,
    input  logic              host_mmio_we_i,
    input  logic [31:0]       host_mmio_wdata_i,
    output logic [31:0]       host_mmio_rdata_o,
    output logic              host_mmio_ack_o,
    output logic              host_intr_o,

    // External memory interface (to GDDR/HBM)
    output logic [39:0]       mem_addr_o,
    output logic              mem_req_o,
    output logic              mem_we_o,
    output logic [511:0]      mem_wdata_o,
    input  logic              mem_ack_i,
    input  logic [511:0]      mem_rdata_i
);

  import tpt_pkg::*;

  // Internal signals
  logic        sched_enable, gpu_reset, gpu_boot;
  logic [5:0]  doorbell_warp;
  logic        doorbell_valid;
  logic [5:0]  active_warp_id;
  logic [31:0] active_warp_pc, active_warp_mask;
  logic [15:0] cta_id_x, cta_id_y, cta_id_z;
  logic [5:0]  num_active_warps;
  logic [NUM_WARPS-1:0] warp_active;
  logic        gpu_ready, gpu_idle, gpu_error;
  logic [63:0] vram_total, vram_free;

  assign vram_total = 64'd(8589934592);
  assign vram_free  = 64'd(8589934592);

  //--------------------------------------------------------------------------
  // CSR (MMIO register block)
  //--------------------------------------------------------------------------
  tpt_csr csr_inst (
      .clk_i              (clk_i),
      .rst_n_i            (rst_n_i),
      .mmio_addr_i        (host_mmio_addr_i),
      .mmio_req_i         (host_mmio_req_i),
      .mmio_we_i          (host_mmio_we_i),
      .mmio_wdata_i       (host_mmio_wdata_i),
      .mmio_rdata_o       (host_mmio_rdata_o),
      .mmio_ack_o         (host_mmio_ack_o),
      .gpu_ready_i        (gpu_ready),
      .gpu_idle_i         (gpu_idle),
      .gpu_error_i        (gpu_error),
      .num_active_warps_i (num_active_warps),
      .vram_total_i       (vram_total),
      .vram_free_i        (vram_free),
      .sched_enable_o     (sched_enable),
      .gpu_reset_o        (gpu_reset),
      .gpu_boot_o         (gpu_boot),
      .doorbell_warp_o    (doorbell_warp),
      .doorbell_valid_o   (doorbell_valid),
      .intr_o             (host_intr_o)
  );

  //--------------------------------------------------------------------------
  // Warp Scheduler
  //--------------------------------------------------------------------------
  logic        warp_stall, warp_done, warp_branch_taken;
  logic [31:0] warp_branch_target;

  tpt_warp_sched sched_inst (
      .clk_i                (clk_i),
      .rst_n_i              (rst_n_i),
      .sched_enable_i       (sched_enable),
      .active_warp_id_o     (active_warp_id),
      .active_warp_pc_o     (active_warp_pc),
      .active_warp_mask_o   (active_warp_mask),
      .active_warp_next_pc_i(active_warp_pc + 32'd4),
      .warp_stall_i         (warp_stall),
      .warp_done_i          (warp_done),
      .warp_branch_taken_i  (warp_branch_taken),
      .warp_branch_target_i (warp_branch_target),
      .dispatch_valid_i     (doorbell_valid),
      .dispatch_warp_id_i   (doorbell_warp),
      .dispatch_pc_i        (32'd0),
      .dispatch_mask_i      (32'hFFFFFFFF),
      .dispatch_cta_id_x_i  (16'd0),
      .dispatch_cta_id_y_i  (16'd0),
      .dispatch_cta_id_z_i  (16'd0),
      .cta_id_x_o           (cta_id_x),
      .cta_id_y_o           (cta_id_y),
      .cta_id_z_o           (cta_id_z),
      .warp_active_o        (warp_active),
      .num_active_warps_o   (num_active_warps)
  );

  //--------------------------------------------------------------------------
  // TPT Processing Core (single SM — replicate for NUM_SM > 1)
  //--------------------------------------------------------------------------
  logic [31:0] core_imem_addr, core_imem_rdata;
  logic        core_imem_req,  core_imem_ack;
  logic [31:0] core_dmem_addr, core_dmem_wdata, core_dmem_rdata;
  logic        core_dmem_req,  core_dmem_we, core_dmem_ack;
  logic [3:0]  core_dmem_be;

  tpt_core core_inst (
      .clk_i         (clk_i),
      .rst_n_i       (rst_n_i & gpu_boot & ~gpu_reset),
      .imem_addr_o   (core_imem_addr),
      .imem_rdata_i  (core_imem_rdata),
      .imem_req_o    (core_imem_req),
      .imem_ack_i    (core_imem_ack),
      .dmem_addr_o   (core_dmem_addr),
      .dmem_req_o    (core_dmem_req),
      .dmem_we_o     (core_dmem_we),
      .dmem_be_o     (core_dmem_be),
      .dmem_wdata_o  (core_dmem_wdata),
      .dmem_ack_i    (core_dmem_ack),
      .dmem_rdata_i  (core_dmem_rdata)
  );

  //--------------------------------------------------------------------------
  // I-Cache
  //--------------------------------------------------------------------------
  logic icache_miss;
  tpt_icache icache_inst (
      .clk_i        (clk_i),
      .rst_n_i      (rst_n_i),
      .addr_i       (core_imem_addr),
      .req_i        (core_imem_req),
      .ack_o        (core_imem_ack),
      .rdata_o      (core_imem_rdata),
      .miss_o       (icache_miss),
      .fill_valid_i (1'b0),  // L2 fill not modeled in simulation
      .fill_addr_i  ('0),
      .fill_data_i  ('0),
      .event_miss_o ()
  );

  //--------------------------------------------------------------------------
  // D-Cache
  //--------------------------------------------------------------------------
  logic dcache_miss;
  tpt_dcache dcache_inst (
      .clk_i        (clk_i),
      .rst_n_i      (rst_n_i),
      .addr_i       (core_dmem_addr),
      .req_i        (core_dmem_req),
      .we_i         (core_dmem_we),
      .be_i         (core_dmem_be),
      .wdata_i      (core_dmem_wdata),
      .ack_o        (core_dmem_ack),
      .rdata_o      (core_dmem_rdata),
      .miss_o       (dcache_miss),
      .fill_valid_i (1'b0),
      .fill_addr_i  ('0),
      .fill_data_i  ('0),
      .wb_valid_o   (),
      .wb_addr_o    (),
      .wb_data_o    (),
      .wb_ack_i     (1'b1),
      .event_miss_o ()
  );

  //--------------------------------------------------------------------------
  // Status generation
  //--------------------------------------------------------------------------
  assign gpu_ready = gpu_boot & ~gpu_reset;
  assign gpu_idle  = (num_active_warps == '0);
  assign gpu_error = 1'b0;

  //--------------------------------------------------------------------------
  // External memory interface (simplified — direct pass-through for sim)
  // In real silicon, this would go through L2 cache and memory controller
  //--------------------------------------------------------------------------
  assign mem_addr_o  = {8'd0, core_dmem_addr};
  assign mem_req_o   = 1'b0;  // Not used in simulation
  assign mem_we_o    = 1'b0;
  assign mem_wdata_o = '0;

  //--------------------------------------------------------------------------
  // Simplified warp pipeline feedback
  // In a full implementation, these would come from the pipeline control
  //--------------------------------------------------------------------------
  assign warp_stall         = 1'b0;
  assign warp_done          = 1'b0;
  assign warp_branch_taken  = 1'b0;
  assign warp_branch_target = '0;

endmodule : tpt_gpu_top