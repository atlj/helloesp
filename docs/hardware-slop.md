# ESP323248S035 Peripheral Reference for Rust Bindings

This document extracts the **peripheral-facing hardware details** from `include/ESP323248S035.hpp` and presents them as a reference for building **Rust bindings or native Rust drivers**.

Only hardware-relevant details are covered here. Framework abstractions, scheduling helpers, and general C++ structure are intentionally omitted unless they affect how a peripheral behaves on the wire.

## Peripheral inventory

The header exposes these board peripherals:

- LCD panel
- Touch controller
- LCD backlight
- RGB LED
- audio amplifier
- light sensor
- microSD card

For Rust work, the most important display-related pieces are:

1. LCD panel interface
2. touch controller interface
3. LCD backlight PWM
4. RGB LED PWM

---

## LCD panel

## Transport

The LCD is driven over **SPI**.

### SPI configuration

| Property | Value |
| --- | --- |
| Bus | `HSPI` |
| Bus ID | `2` |
| Clock | `80_000_000` Hz |
| Bit order | MSB first |
| SPI mode | Mode 0 |

### LCD pins

| Signal | GPIO |
| --- | ---: |
| MISO | 12 |
| MOSI | 13 |
| SCLK | 14 |
| CS | 15 |
| DC / RS | 2 |

## Panel geometry and pixel format

| Property | Value |
| --- | --- |
| Raw panel width | `320` |
| Raw panel height | `480` |
| Color depth | `16` bpp |
| Pixel format | RGB565 |

The header uses:

- `colmod = 0x55` effectively via `rgb16 | ctrl16`
- pixel size = 2 bytes

So a Rust driver should treat panel writes as **RGB565 / 16-bit** data.

## Panel controller assumptions

The initialization sequence is labeled as an **ST7796 initialization sequence**, so the display is assumed to be compatible with that command set.

### Commands used

The C++ header defines these LCD commands:

| Name | Value |
| --- | ---: |
| `swreset` | `0x01` |
| `slpin` | `0x10` |
| `slpout` | `0x11` |
| `noron` | `0x13` |
| `invoff` | `0x20` |
| `dispon` | `0x29` |
| `caset` | `0x2A` |
| `raset` | `0x2B` |
| `ramwr` | `0x2C` |
| `madctl` | `0x36` |
| `colmod` | `0x3A` |
| `pgc` | `0xE0` |
| `ngc` | `0xE1` |
| `cscon` | `0xF0` |

These are enough to reproduce the panel bring-up logic in Rust.

## Data/command protocol

The panel uses a dedicated **DC** pin:

- `DC = LOW` for command
- `DC = HIGH` for data

The transaction pattern is:

1. drive `DC` low
2. begin SPI transaction
3. assert `CS` low
4. write command byte
5. if payload exists, drive `DC` high and write payload bytes
6. deassert `CS`
7. end SPI transaction

For Rust, this means your LCD abstraction needs:

- SPI write access
- GPIO output control for `DC`
- GPIO output control for `CS`

## Initialization sequence

The panel init flow in the header is:

1. `swreset`
2. delay 100 ms
3. `cscon 0xC3`
4. `cscon 0x96`
5. `colmod <rgb565>`
6. `madctl <orientation | subpixel order>`
7. positive gamma table
8. negative gamma table
9. `cscon 0x3C`
10. `cscon 0x69`
11. `invoff`
12. `noron`
13. `slpout`
14. `dispon`

### Gamma payloads

Positive gamma:

```text
F0 09 0B 06 04 15 2F 54 42 3C 17 14 18 1B
```

Negative gamma:

```text
E0 09 0B 06 04 03 2B 43 42 3B 16 14 17 1B
```

Those payloads should be copied directly if you want Rust behavior to match the current implementation.

## Orientation

The header defines these `MADCTL` bits:

| Bit name | Value |
| --- | ---: |
| `MY` | `0x80` |
| `MX` | `0x40` |
| `MV` | `0x20` |
| `BGR` | `0x08` |
| `RGB` | `0x00` |

The selected defaults are:

- subpixel order: `RGB`
- aspect/orientation: `wide_inverted`

`wide_inverted` is defined as:

```text
MY | MV | MX = 0xE0
```

So the effective `MADCTL` value written is:

```text
0xE0 | 0x00 = 0xE0
```

### Logical dimensions

The code swaps logical width/height when the `MV` rotation bit is active.

That means the exported logical display size is:

- width = `480`
- height = `320`

even though the physical panel constants are `320 x 480`.

If you want Rust code to match the current C++ behavior, treat the **runtime drawing surface as 480x320** with the current default rotation.

## Flush/write model

Pixel flushing is done by:

1. issuing `CASET` with x start/end
2. issuing `RASET` with y start/end
3. issuing `RAMWR`
4. streaming pixel bytes

In Rust, the equivalent low-level API would likely be:

- `set_window(x1, y1, x2, y2)`
- `write_pixels(&[u8])`

The byte order is whatever the panel expects for RGB565 writes; the C++ code sends the provided pixel buffer directly without per-pixel transformation in the flush path.

---

## Touch controller

## Transport

The touch controller is on **I2C**.

### I2C configuration

| Property | Value |
| --- | --- |
| Bus | `Wire1` |
| Bus ID | `1` |
| Device address | `0x5D` |

### Touch pins

| Signal | GPIO |
| --- | ---: |
| SCL | 32 |
| SDA | 33 |
| INT | 21 |
| RST | 25 |

The header only actively initializes SDA/SCL in the visible code path. `INT` and `RST` are declared but not configured in the shown init path.

## Register map used

| Register | Address | Meaning |
| --- | ---: | --- |
| Product ID | `0x8140` | 4-byte identification field |
| Status | `0x814E` | data-ready and touch count |
| First point | `0x814F` | first touch-point record |

## Register access protocol

The driver uses a **16-bit register address**:

1. write register MSB
2. write register LSB
3. for reads, perform repeated-start style access and request bytes

This is implemented in the header by:

- writing the 16-bit register address first
- ending transmission with `false` before `requestFrom(...)`

For Rust, this implies the controller expects a common pattern like:

```text
write(addr, [reg_hi, reg_lo])
read(addr, buf)
```

or a combined transaction if your HAL supports it.

## Touch point structure

Each touch point record is defined as:

| Field | Type |
| --- | --- |
| `track` | `u8` |
| `x` | `u16` |
| `y` | `u16` |
| `area` | `u16` |
| padding | `u8` |

The structure is packed and totals **8 bytes**.

The comments indicate the controller stores up to **5 touch points** in consecutive memory slots:

1. `0x814F-0x8156`
2. `0x8157-0x815E`
3. `0x815F-0x8166`
4. `0x8167-0x816E`
5. `0x816F-0x8176`

## Touch count behavior

The status register logic works like this:

- bit 7 indicates data ready
- low nibble contains the active touch count

The header then:

1. reads the status byte
2. checks `count & 0x80`
3. checks the low nibble is below `_touch_max`
4. clears the status register by writing back zero
5. returns `count & 0x0F`

### Important behavior detail

The implementation acknowledges a touch sample by clearing the status register after reading it.

If you reproduce this in Rust, do not forget the clear step or touch samples may stop advancing as expected.

## Multi-touch vs current software behavior

Hardware capability in the header comments:

- up to **5 simultaneous touch points**

Current software behavior:

- only **1 touch point** is exposed to the graphics/input layer

So for Rust you have two realistic options:

1. match existing behavior and implement single-touch only
2. expose all touch points and build a richer interface than the C++ code currently uses

## Touch coordinate remapping

After reading raw touch points, the code remaps coordinates based on the active display rotation.

With the current orientation constant, touch data is transformed so it matches the rotated LCD coordinate system.

This is important for Rust bindings:

- raw touch coordinates are **not** directly used
- they are post-processed to match the panel orientation

If you want behavioral parity, your Rust touch driver should apply the same mapping as the current `map(...)` logic.

## Calibration status

The header includes a:

```text
TODO: calibrate touch?
```

So there is no explicit calibration matrix or offset/scale calibration in the current implementation.

That means the current code relies on:

- controller native coordinates
- orientation remapping only

---

## LCD backlight

The LCD backlight is controlled separately from the LCD panel transfer path.

## PWM configuration

| Property | Value |
| --- | --- |
| GPIO | 27 |
| PWM channel | 12 |
| Frequency | 5 kHz |
| Resolution | 8-bit |
| Max duty | 255 |

## Behavior

Backlight brightness is controlled with:

```cpp
set_backlight(uint16_t duty)
```

which calls:

```cpp
ledcWrite(channel, duty)
```

The current initialization sets the backlight to full scale immediately after panel init.

For Rust, this can be modeled as a simple PWM output with duty range:

```text
0..=255
```

---

## RGB LED

The board also has a dedicated RGB LED peripheral implemented with PWM.

## Pin and PWM mapping

| Color | GPIO | PWM channel |
| --- | ---: | ---: |
| Red | 4 | 13 |
| Green | 16 | 14 |
| Blue | 17 | 15 |

## PWM configuration

| Property | Value |
| --- | --- |
| Frequency | 5 kHz |
| Resolution | 8-bit |
| Max duty | 255 |

## Output behavior

The write path is:

```cpp
ledcWrite(r_chan, 255 - rgb.red);
ledcWrite(g_chan, 255 - rgb.green);
ledcWrite(b_chan, 255 - rgb.blue);
```

This means the RGB LED channels are **inverted**.

### Likely implication

The LED hardware is probably **active-low**:

- lower duty output corresponds to higher logical channel intensity in the code
- full off/on semantics are inverted relative to a straightforward active-high LED

If you implement this in Rust, preserve that inversion unless you confirm the electrical design is different.

## State model

The C++ code keeps:

- `_rgb`: last applied color
- `_set`: next requested color

That is just a staging optimization. A Rust implementation does not need to copy that exact API unless you want update-loop parity with the original code.

From a peripheral perspective, the essential behavior is simply:

- three PWM outputs
- 8-bit color channels
- inverted duty mapping

---

## Minimal Rust-facing hardware constants

If you want a concise set of constants to carry into Rust first, these are the important ones.

## LCD

```text
SPI bus: HSPI / 2
SCLK: 14
MISO: 12
MOSI: 13
CS: 15
DC: 2
SPI freq: 80_000_000
SPI mode: 0
Panel raw size: 320x480
Logical default size: 480x320
Pixel format: RGB565
```

## Touch

```text
I2C bus: Wire1 / 1
SCL: 32
SDA: 33
INT: 21
RST: 25
Address: 0x5D
Product ID reg: 0x8140
Status reg: 0x814E
First point reg: 0x814F
Max hardware points: 5
```

## Backlight

```text
GPIO: 27
PWM channel: 12
PWM freq: 5000
PWM bits: 8
```

## RGB LED

```text
R: GPIO 4, PWM 13
G: GPIO 16, PWM 14
B: GPIO 17, PWM 15
PWM freq: 5000
PWM bits: 8
Inverted output: yes
```

---

## Practical binding priorities

If the goal is to create Rust support incrementally, the header suggests this order:

1. implement LCD SPI command/data writes
2. reproduce the panel init sequence
3. implement rectangular pixel flushes
4. add PWM control for backlight
5. implement I2C reads for touch status and point data
6. add orientation-aware touch coordinate remapping
7. add RGB LED PWM control

That sequence matches the current hardware dependencies and keeps the display usable before touch is fully integrated.
