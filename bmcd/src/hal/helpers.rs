// Copyright 2023 Turing Machines
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use gpiocdev::line::Value;
use gpiocdev::Request;
use std::collections::HashMap;
use std::path::Path;
const NODE_COUNT: u8 = 4;

/// Wrapper around gpiocdev::Request that also stores the line offsets
pub struct GpioLines {
    request: Request,
    offsets: Vec<u32>,
}

impl GpioLines {
    pub fn new(request: Request, offsets: Vec<u32>) -> Self {
        Self { request, offsets }
    }

    /// Set a single value (for single-line requests)
    pub fn set_value(&self, value: Value) -> gpiocdev::Result<()> {
        self.request.set_value(self.offsets[0], value)
    }

    /// Set values from a bitfield (each bit corresponds to a line)
    pub fn set_values_from_bits(&self, bits: u8) -> gpiocdev::Result<()> {
        for (i, &offset) in self.offsets.iter().enumerate() {
            let value = if (bits >> i) & 1 != 0 {
                Value::Active
            } else {
                Value::Inactive
            };
            self.request.set_value(offset, value)?;
        }
        Ok(())
    }
}

/// small helper macro which handles the code duplication of declaring gpio lines.
#[macro_export]
macro_rules! gpio_output_lines {
    ($chip:expr, $output:expr) => {{
        let offsets: Vec<u32> = $output.iter().copied().collect();
        let request = gpiocdev::Request::builder()
            .on_chip($chip)
            .with_lines(&$output)
            .as_output(gpiocdev::line::Value::Inactive)
            .request()
            .context(concat!("error initializing pin ", stringify!($output)))?;
        $crate::hal::helpers::GpioLines::new(request, offsets)
    }};
}

/// uses [`gpio_output_lines`] to declare an array of `GpioLines` objects
#[macro_export]
macro_rules! gpio_output_array {
    ($chip:expr, $($pin:ident),+) => {
       [
           $(
               $crate::gpio_output_lines!($chip, [$pin])
           ),*
       ]
    };
}

/// Helper function that converts a bitfield + mask into an iterator. This
/// iterator iterates over each bit, and skips the bits that are not set in the
/// nodes_mask.
///
/// # Arguments
///
/// * `node_states`     bit-field where each bit represents a node on the
///     turing-pi board, if bit(n) = 1 equals 'select' and bit(n) = 0 equals
///     'unselect'.
/// * `node_mask`       mask which bits to select.
///
/// # Returns
///
/// iterator returns a tuple containing the index of a bit + the new value.  
pub fn bit_iterator(nodes_state: u8, nodes_mask: u8) -> impl Iterator<Item = (usize, u8)> {
    (0..NODE_COUNT).filter_map(move |n| {
        let mask = nodes_mask & (1 << n);
        let state = (nodes_state & mask) >> n;
        (mask != 0).then_some((n as usize, state))
    })
}

pub fn load_lines<P: AsRef<Path>>(chip: P) -> HashMap<String, u32> {
    let chip_ref = gpiocdev::chip::Chip::from_path(chip.as_ref()).expect("failed to open chip");
    HashMap::from_iter((0..chip_ref.info().expect("chip info").num_lines).filter_map(|i| {
        chip_ref.line_info(i).ok().and_then(|info| {
            let name = info.name.to_string();
            if name.is_empty() {
                None
            } else {
                Some((name, i))
            }
        })
    }))
}
