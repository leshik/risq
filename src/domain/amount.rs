use std::cmp::Ordering;
use std::ops::*;

#[derive(Eq, Clone, Debug, Copy)]
pub struct NumberWithPrecision {
    base_amount: u64,
    precision: u32,
}

impl NumberWithPrecision {
    pub fn new(base_amount: u64, precision: u32) -> Self {
        Self {
            base_amount,
            precision,
        }
    }

    pub fn format(&self, target_precision: u32) -> String {
        let mut ret = String::new();
        let mut rest_amount = self.base_amount;

        if target_precision > self.precision {
            rest_amount = rest_amount * 10_u64.pow(target_precision - self.precision);
        } else if self.precision > target_precision {
            rest_amount = rest_amount / 10_u64.pow(self.precision - target_precision);
        }

        while ret.len() < target_precision as usize {
            ret.push(char_of_last_digit(rest_amount));
            rest_amount = rest_amount / 10;
        }
        ret.push('.');

        while rest_amount > 0 {
            ret.push(char_of_last_digit(rest_amount));
            rest_amount = rest_amount / 10;
        }
        if ret.len() == target_precision as usize + 1 {
            ret.push('0');
        }
        ret.chars().rev().collect()
    }

    fn with_precision(&self, target_precision: u32) -> Self {
        let mut rest_amount = self.base_amount;
        if target_precision > self.precision {
            rest_amount = rest_amount * 10_u64.pow(target_precision - self.precision);
        } else if self.precision > target_precision {
            rest_amount = rest_amount / 10_u64.pow(self.precision - target_precision);
        }
        Self::new(rest_amount, target_precision)
    }
}

fn char_of_last_digit(n: u64) -> char {
    match n % 10 {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        _ => unimplemented!(),
    }
}

impl Mul for NumberWithPrecision {
    type Output = Self;

    fn mul(self, right: Self) -> Self {
        let (left_precision, right_precision) = (self.precision, right.precision);
        let (mut left_value, mut right_value) = (self.base_amount, right.base_amount);
        let mut res_precision = left_precision + right_precision;
        let target_precision = u32::max(left_precision, right_precision);

        while res_precision > target_precision {
            if left_value % 10 == 0 {
                left_value = left_value / 10;
                res_precision -= 1;
            } else {
                break;
            }
        }
        while res_precision > target_precision {
            if right_value % 10 == 0 {
                right_value = right_value / 10;
                res_precision -= 1;
            } else {
                break;
            }
        }
        let mut res = left_value * right_value;
        if res_precision > target_precision {
            res = res / 10_u64.pow(res_precision - right_precision);
        } else if res_precision < target_precision {
            res = res * 10_u64.pow(right_precision - res_precision);
        }
        NumberWithPrecision::new(res, target_precision)
    }
}
impl Div<u64> for NumberWithPrecision {
    type Output = Self;

    fn div(self, rhs: u64) -> Self::Output {
        if rhs == 0 {
            panic!("Cannot divide by zero-valued `NumberWithPrecision`!");
        }
        NumberWithPrecision::new(self.base_amount / rhs, self.precision)
    }
}

impl PartialEq for NumberWithPrecision {
    fn eq(&self, other: &NumberWithPrecision) -> bool {
        let target_precision = self.precision.max(other.precision);
        self.with_precision(target_precision).base_amount
            == other.with_precision(target_precision).base_amount
    }
}
impl PartialOrd for NumberWithPrecision {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let target_precision = self.precision.max(other.precision);
        Some(
            self.with_precision(target_precision)
                .base_amount
                .cmp(&other.with_precision(target_precision).base_amount),
        )
    }
}
impl Ord for NumberWithPrecision {
    fn cmp(&self, other: &Self) -> Ordering {
        let target_precision = self.precision.max(other.precision);
        self.with_precision(target_precision)
            .base_amount
            .cmp(&other.with_precision(target_precision).base_amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiply() {
        let left_precision = 4;
        let right_precision = 8;

        let price_base = 9000 * 10_u64.pow(left_precision);
        let amount_base = 1 * 10_u64.pow(right_precision);
        let price = NumberWithPrecision::new(price_base, left_precision);
        let amount = NumberWithPrecision::new(amount_base, right_precision);

        let high_volume = price * amount;
        let low_volume = price * amount / 10000;
        assert!(&high_volume.format(8) == "9000.00000000");
        assert!(&low_volume.format(8) == "0.90000000");
    }
}
