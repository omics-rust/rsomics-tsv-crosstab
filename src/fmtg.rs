// Reproduce C `printf("%.14g", x)` byte-for-byte: that is the format datamash
// emits its numeric results in, so matching it is what makes the table
// byte-identical. The %g rule: with P significant digits, use %e when the
// decimal exponent is < -4 or >= P, otherwise %f; then strip trailing zeros
// and a bare trailing point.
const P: usize = 14;

#[cfg(test)]
fn g14(x: f64) -> Vec<u8> {
    let mut out = Vec::with_capacity(24);
    write_g(x, &mut out);
    out
}

pub fn write_g(x: f64, out: &mut Vec<u8>) {
    if x.is_nan() {
        out.extend_from_slice(b"nan");
        return;
    }
    if x.is_infinite() {
        out.extend_from_slice(if x < 0.0 { b"-inf" } else { b"inf" });
        return;
    }
    if x == 0.0 {
        // Preserve the sign of -0.0 the way printf does.
        if x.is_sign_negative() {
            out.push(b'-');
        }
        out.push(b'0');
        return;
    }

    if x < 0.0 {
        out.push(b'-');
    }
    let a = x.abs();

    // %e with P-1 fractional digits gives the rounded significand + exponent;
    // the exponent then decides %e vs %f per the %g rule.
    let e_str = format!("{:.*e}", P - 1, a);
    let (mantissa, exp) = split_e(&e_str);
    let exp: i32 = exp.parse().unwrap();

    if exp < -4 || exp >= P as i32 {
        emit_e(&mantissa, exp, out);
    } else {
        emit_f(&mantissa, exp, out);
    }
}

// Rust's {:e} yields e.g. "1.6666666666667e0"; return (digits-without-point, exp).
fn split_e(s: &str) -> (Vec<u8>, String) {
    let bytes = s.as_bytes();
    let epos = bytes.iter().position(|&b| b == b'e').unwrap();
    let exp = s[epos + 1..].to_string();
    let mut digits = Vec::with_capacity(P);
    for &b in &bytes[..epos] {
        if b != b'.' {
            digits.push(b);
        }
    }
    (digits, exp)
}

fn trim_zeros(digits: &mut Vec<u8>) {
    while digits.len() > 1 && *digits.last().unwrap() == b'0' {
        digits.pop();
    }
}

fn emit_e(mantissa: &[u8], exp: i32, out: &mut Vec<u8>) {
    let mut d = mantissa.to_vec();
    trim_zeros(&mut d);
    out.push(d[0]);
    if d.len() > 1 {
        out.push(b'.');
        out.extend_from_slice(&d[1..]);
    }
    out.push(b'e');
    out.push(if exp < 0 { b'-' } else { b'+' });
    let e = exp.unsigned_abs();
    // printf pads the exponent to at least two digits.
    let es = format!("{e:02}");
    out.extend_from_slice(es.as_bytes());
}

fn emit_f(mantissa: &[u8], exp: i32, out: &mut Vec<u8>) {
    // mantissa is P significant digits (no point); the decimal point sits after
    // position exp+1.
    let d = mantissa.to_vec();
    let point = exp + 1; // count of digits before the decimal point
    if point <= 0 {
        // 0.00…digits
        let lead = (-point) as usize;
        let mut frac = vec![b'0'; lead];
        frac.extend_from_slice(&d);
        trim_zeros(&mut frac);
        out.push(b'0');
        out.push(b'.');
        out.extend_from_slice(&frac);
    } else {
        let point = point as usize;
        if point >= d.len() {
            // integer value: pad with zeros, no fractional part.
            out.extend_from_slice(&d);
            for _ in 0..(point - d.len()) {
                out.push(b'0');
            }
        } else {
            out.extend_from_slice(&d[..point]);
            let mut frac = d[point..].to_vec();
            trim_zeros(&mut frac);
            if frac != [b'0'] {
                out.push(b'.');
                out.extend_from_slice(&frac);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::g14;

    fn s(x: f64) -> String {
        String::from_utf8(g14(x)).unwrap()
    }

    #[test]
    fn matches_printf_14g() {
        assert_eq!(s(1.5), "1.5");
        assert_eq!(s(2.0), "2");
        assert_eq!(s(3.75), "3.75");
        assert_eq!(s(3000000.0), "3000000");
        assert_eq!(s(5.0 / 3.0), "1.6666666666667");
        assert_eq!(s(2.0 / 3.0), "0.66666666666667");
        assert_eq!(s(6.0), "6");
        assert_eq!(s(0.3), "0.3");
        assert_eq!(s(1.25), "1.25");
        assert_eq!(s(123456789012345.0), "1.2345678901234e+14");
        assert_eq!(s(1e14), "1e+14");
        assert_eq!(s(99999999999999.0 + 1.0), "1e+14");
        assert_eq!(s(0.0), "0");
        assert_eq!(s(-2.5), "-2.5");
        assert_eq!(s(0.0001), "0.0001");
        assert_eq!(s(0.00001), "1e-05");
    }
}
