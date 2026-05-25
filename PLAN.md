# 3D Game Engine Prototype Architecture Plan (macOS Target)

This document outlines the detailed development plan for building a highly modular, data-driven, memory-safe game engine using **Rust**, **wgpu** (targeting native **Metal** on Apple Silicon), and an **Archetype Entity Component System (ECS)**.

---

## Phase 1: Foundations & OS Integration
**Goal:** Establish a memory-safe execution loop and connect the engine logic to the macOS environment without any graphical rendering.

### Task 1.1: Multi-Crate Workspace Architecture Setup
* **Objective:** Define physical boundaries for engine code to prevent compile-time inflation and enforce strict separation of concerns.
* **Subtasks:**
  * Initialize a Cargo workspace containing four distinct sub-crates: `core`, `renderer`, `ecs_world`, and `editor`.
  * Configure dependency directions: `core` depends on `renderer` and `ecs_world`. `editor` depends on `core`. The `renderer` and `ecs_world` crates must remain entirely agnostic of each other.
  * Establish global logging and diagnostic systems using structural tracking macros to capture platform-specific engine initialization timings.

### Task 1.2: Window Lifecycle and OS Event Loop
* **Objective:** Bind the engine to the Cocoa window framework on macOS using `winit` and establish a deterministic execution heart rate.
* **Subtasks:**
  * Configure the OS loop to leverage low-power state polling when the engine window loses focus on macOS.
  * Abstract raw application window creation with explicit configuration for high-DPI Retina displays, forcing logical coordinate translation to physical pixel boundaries.
  * Intercept native platform quit, resize, and scale-factor change requests, transforming them into internal engine event variants passed to downstream sub-crates.

### Task 1.3: Deterministic Engine Clock & Fixed Timestep Ticker
* **Objective:** Build a high-precision game loop time provider resistant to frame-rate hitching and variable refresh rates (Apple ProMotion displays).
* **Subtasks:**
  * Implement a high-resolution monotonic clock wrapper tracking delta time (the duration between individual frames).
  * Isolate the update tick into two states: a **Fixed Timestep Loop** running at a constant frequency (e.g., exactly 60Hz) for simulation/physics consistency, and a **Variable Frame Loop** for interpolation and rendering logic.
  * Build a time accumulation buffer that handles fractional time remaining between physics ticks to prevent visual jitter.

---

## Phase 2: Core Data Architecture & Memory Layout
**Goal:** Build the data storage foundation of the engine. Organize all game data linearly in RAM to maximize CPU cache efficiency.

### Task 2.1: Archetype ECS Space Definition
* **Objective:** Implement the fundamental layout of the Entity Component System (`flecs_ecs` integration) to act as the single source of truth for engine state.
* **Subtasks:**
  * Register core foundational component structures: `Transform3D` (SIMD matrix), `Velocity3D` (velocity vector), and `RenderMeshReference` (resource identifier).
  * Verify structural memory alignment of components using explicit data layout attributes (`#[repr(C)]`) to guarantee memory predictability.
  * Write structural integration pipelines enabling the `core` crate to create, delete, and modify entity definitions globally across execution ticks.

### Task 2.2: Cache-Aligned Query & System Processing
* **Objective:** Establish parallelizable data iteration pipelines that loop through matching entity types.
* **Subtasks:**
  * Construct execution queries that isolate combinations of components (e.g., entities possessing both a `Transform3D` and a `Velocity3D`).
  * Build a dedicated movement execution system that processes spatial translation linearly through memory blocks, avoiding pointer indirection.
  * Expose safe, multi-threaded access parameters allowing systems to run safely across multiple CPU core threads without triggering memory lock synchronization traps.

---

## Phase 3: Hardware-Accelerated Graphics Pipeline
**Goal:** Connect the OS window handle directly to a low-overhead GPU presentation surface and drive Apple Silicon GPUs via native Metal commands through `wgpu`.

### Task 3.1: Modern Adapter and Surface Context Binding
* **Objective:** Bind the window handle to the graphics layer with zero-copy parameters.
* **Subtasks:**
  * Instantiate the graphics runtime interface explicitly targeting modern API backends (forcing Metal translation layers on macOS).
  * Query the connected display for supported texture profiles, opting for modern color accuracy standards (e.g., BGRA unnormalized sRGB color space).
  * Configure surface parameters to support a maximum back-buffer frame latency of 2, preventing input lag while maintaining a steady queue of instructions to the GPU.

### Task 3.2: Render Pipeline State Formulation
* **Objective:** Build the fixed-function and programmable GPU pipeline state objects required to draw 3D graphics.
* **Subtasks:**
  * Author a baseline vertex and fragment shader using WebGPU Shading Language (WGSL), optimized for Apple Silicon's execution model.
  * Define the vertex input assembly layouts, specifying exactly how raw byte buffers correspond to spatial coordinates, textures, and normal mapping data.
  * Configure blending operations, culling behavior (e.g., discarding back-facing polygons), and depth/stencil testing states to prevent visual rendering bugs in 3D spaces.

### Task 3.3: GPU Buffer Allocator & Asset Streamer
* **Objective:** Establish high-speed host-to-device memory copy operations.
* **Subtasks:**
  * Construct a central mesh storage allocator that manages GPU-side vertex buffers and index pools.
  * Implement zero-copy structure marshalling (`bytemuck` mapping) to safely project raw CPU component matrices into GPU-readable data layouts.
  * Build a uniform buffer storage mechanism that updates global scene variables (like camera view matrices and projection angles) once per frame execution pass.

---

## Phase 4: The Data-to-GPU Binding Layer
**Goal:** Build the runtime bridge that translates live data sitting inside the ECS into compact command packets for the GPU.

### Task 4.1: Render Queue Synchronization System
* **Objective:** Create a dedicated bridge system that translates live entity components into compact draw parameters right after the physics simulation pass.
* **Subtasks:**
  * Implement a synchronization system in the `core` crate that executes right after the physics simulation pass.
  * Query the ECS universe for all active entities possessing valid spatial data and active rendering references.
  * Extract their world matrices and instance IDs, copying them into a single, flat, contiguous vector of memory allocations tailored for instanced rendering passes.

### Task 4.2: Fully Instanced Draw Command Batching
* **Objective:** Maximize GPU efficiency by reducing driver overhead and drawing thousands of identical objects in a single draw instruction.
* **Subtasks:**
  * Group collected render data chunks by their underlying mesh assets to avoid binding different mesh objects multiple times per frame.
  * Allocate a dynamic Instance Buffer on the GPU that scales smoothly depending on the current count of active world entities.
  * Write a single rendering encoding routine that binds the primary mesh data once, maps the Instance Buffer data, and executes an instanced draw command to render all matching entities simultaneously.

---

## Phase 5: Editor Tooling & Interactive Shell
**Goal:** Add an immediate-mode interactive GUI overlay on top of your live engine state to transform the headless runtime into an authoring tool.

### Task 5.1: Overlay GUI Frame Integration
* **Objective:** Render an immediate-mode user interface cleanly overlaid across your engine's viewport without corrupting 3D game rendering passes using `egui`.
* **Subtasks:**
  * Integrate a canvas render wrapper configured to process native platform inputs (mouse movements, keystrokes) handed down from your OS window events.
  * Initialize a second, separate render pipeline pass that captures the UI vertex output and displays it directly onto the final screen texture.
  * Incorporate automated interface UI scaling calculation that adapts cleanly when moving the engine editor between standard and high-DPI displays.

### Task 5.2: Live Entity Inspector and World Tree Node Tree
* **Objective:** Construct interactive editor tools to monitor internal engine memory spaces visually.
* **Subtasks:**
  * Build a world tree UI panel that pulls active entity IDs from the ECS and displays them as an interactive list.
  * Implement a property inspector panel that dynamically exposes field data values (like position offsets or scale floats) of a selected entity.
  * Connect UI input actions back to the underlying data stores, allowing real-time adjustments made on slider menus to instantly update the values stored in the ECS tables.

---

## Prototype Milestone Definition: "The Stress Test"
The prototype phase is considered successful when it meets the following operational criteria:
1. **Memory Cleanliness:** An empty project builds down to a minimal binary footprint and consumes less than 50MB of RAM at runtime.
2. **Performance Scale:** The engine runs smoothly at a stable frame rate on your Mac hardware while simulating and drawing 10,000 distinct 3D instances simultaneously.
3. **Hot Adjustment:** Modifying a coordinate slider inside your Editor panel instantly moves the target 3D shape across the screen in real-time, without hitching or memory allocation stalls.
