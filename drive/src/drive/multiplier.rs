use crate::error::drive::DriveError::{MultiplierEncodingNotSupported, MultiplierNotSupported};
use crate::error::Error;
use std::ops::Div;

// The multiplier is encoded on a single byte, of which the first bit is reserved
pub struct Multiplier {
    byte: u8,
}

impl Multiplier {
    pub fn from_byte(byte: u8) -> Result<Self, Error> {
        if byte & 0x80 != 0 {
            Err(Error::Drive(MultiplierNotSupported(
                "Multipliers have the first bit reserved for future use",
            )))
        } else {
            Ok(Multiplier { byte })
        }
    }

    // We are trying to encode 7 bits to a float.
    //
    // The requirements are the following:
    // * The multiplier should be a rational number that is also a terminating decimal number.
    //   (i.e 0.25, not 0.3333333...)
    // * The original multiplier of 1 should equate to roughly at cost for a Dash price of 100 dollars.
    // * The multiplier should be easily understandable to follow the Dash price
    //
    // V1 (at release) will not let the masternode network change the multiplier.
    //
    // In v1 the base multiplier will be chosen to be roughly twice the cost to the network.
    // Example recommended values:
    // * 1 Dash = 10$, cost 10, multiplier 0.2
    // * 1 Dash = 50$, cost 2, multiplier 1
    // * 1 Dash = 100$, cost 1, multiplier 2
    // * 1 Dash = 200$, cost 0.5, multiplier 4
    // * 1 Dash = 500$, cost 0.2, multiplier 10
    // * 1 Dash = 2,000$, cost 0.05, multiplier 40
    // * 1 Dash = 5,000$, cost 0.02, multiplier 100
    // * 1 Dash = 10,000$, cost 0.01, multiplier 200
    // * 1 Dash = 20,000$, cost 0.005, multiplier 400
    // * 1 Dash = 40,000$, cost 0.0025, multiplier 800
    // * 1 Dash = 200,000$, cost 0.0005, multiplier 4000
    // * 1 Dash = 500,000$, cost 0.00025, multiplier 10000
    // * 1 Dash = 1,000,000$, cost 0.0001, multiplier 20000
    //
    //
    // Solution :
    // * The lowest allowed multiplier would be 0.2, which would make sense at a Dash price of 10$.
    // * The highest allowed multiplier would be 20000, which would make sense at a Dash price of 1 Million $.
    // * Between [0.2 and 2] the minimal step is 0.2 (10$). This gives 10 possible values.
    // * Between ]2 and 10] the minimal step is 0.4 (20$). This gives us 20 possible values.
    // * Between ]10 and 40] the minimal step is 1 (50$). This gives us 30 possible values.
    // * Between ]40 and 200] the minimal step is 8 (400$). This gives us 20 possible values.
    // * Between ]200 and 1000] the minimal step is 40 (2,000$). This gives us 20 possible values.
    // * Between ]1000 and 4000] the minimal step is 200 (10,000$). This gives us 15 possible values.
    // * Between ]4000 and 10000] the minimal step is 1000 (50,000$). This gives us 6 possible values.
    // * Between ]10000 and 20000] the minimal step is 2000 (100,000$). This gives us 5 possible values.
    // This adds up to 126 possible values.

    // A byte value of 0111 1111 (127) is reserved.

    fn multiplier_value(&self) -> Result<f64, Error> {
        let fbyte = self.byte as f64;
        match self.byte {
            // * Between [0.2 and 2] the minimal step is 0.2 (10$). This gives 10 possible values.
            0..=9 => Ok(0.2 + fbyte * 0.2),
            // * Between ]2 and 10] the minimal step is 0.4 (20$). This gives us 20 possible values.
            10..=29 => Ok(2.0 + (fbyte - 10.0) * 0.4),
            // * Between ]10 and 40] the minimal step is 1 (50$). This gives us 30 possible values.
            30..=59 => Ok(10.0 + fbyte - 30.0),
            // * Between ]40 and 200] the minimal step is 8 (400$). This gives us 20 possible values.
            60..=79 => Ok(40.0 + (fbyte - 60.0) * 8.0),
            // * Between ]200 and 1000] the minimal step is 40 (2,000$). This gives us 20 possible values.
            80..=99 => Ok(200.0 + (fbyte - 80.0) * 40.0),
            // * Between ]1000 and 4000] the minimal step is 200 (10,000$). This gives us 15 possible values.
            100..=114 => Ok(1000.0 + (fbyte - 100.0) * 200.0),
            // * Between ]4000 and 10000] the minimal step is 1000 (50,000$). This gives us 6 possible values.
            115..=120 => Ok(4000.0 + (fbyte - 115.0) * 1000.0),
            // * Between ]10000 and 20000] the minimal step is 2000 (100,000$). This gives us 5 possible values.
            120..=124 => Ok(10000.0 + (fbyte - 120.0) * 2000.0),
            _ => Err(Error::Drive(MultiplierEncodingNotSupported(
                "Value not supported",
            ))),
        }
    }

    fn byte_value_for_price(price: u64) -> Result<u8, Error> {
        match price {
            // * Smallest value.
            0..=19 => Ok(0),
            // * Between [20 and 100[ the minimal step is 10$. This gives 9 possible values.
            20..=99 => Ok(((price - 20).div(10) + 1) as u8),
            // * Between [100 and 500[ the minimal step is 20$. This gives us 20 possible values.
            100..=499 => Ok(((price - 100).div(20) + 10) as u8),
            // * Between [500 and 2k[ the minimal step is 50$. This gives us 30 possible values.
            500..=1999 => Ok(((price - 500).div(50) + 30) as u8),
            // * Between [2k and 10k[ the minimal step is 400$. This gives us 20 possible values.
            2000..=9999 => Ok(((price - 2000).div(400) + 60) as u8),
            // * Between [10k and 40k[ the minimal step is 2k$. This gives us 20 possible values.
            10000..=39999 => Ok(((price - 10000).div(2000) + 80) as u8),
            // * Between [40k and 200k[ the minimal step is 10k$. This gives us 15 possible values.
            40000..=199999 => Ok(((price - 40000).div(10000) + 100) as u8),
            // * Between [200k and 10k the minimal step is 50k$. This gives us 6 possible values.
            200000..=499999 => Ok(((price - 200000).div(50000) + 115) as u8),
            // * Between ]10000 and 20000] the minimal step is 2000 (100,000$). This gives us 5 possible values.
            500000..=999999 => Ok(((price - 500000).div(100000) + 121) as u8),
            _ => Err(Error::Drive(MultiplierEncodingNotSupported(
                "Value not supported",
            ))),
        }
    }

    pub fn multiplier_for_price(price: u64) -> Result<Self, Error> {
        Ok(Multiplier {
            byte: Self::byte_value_for_price(price)?,
        })
    }

    pub fn multiply_fee(&self, fee: u64) -> u64 {
        todo!()
    }
}
