use crate::iccprofile::{Curve, Data, ICCNumber, MultiLocalizedUnicodeType};
use std::fmt::{self, Write as FmtWrite};
use crate::DecodedICCProfile;
use bin_rs::io::*;
use crate::ICCProfile;
use std::fs::File;
use std::io::Result;
use std::io::{Error,ErrorKind};
use std::io::prelude::*;

pub(crate) fn bound_check(buf:&[u8],ptr:usize,size:usize) -> Result<()> {
    if ptr > buf.len() || size > buf.len() - ptr {
        return Err(Error::other(format!(
            "Outbound check error len {} but offset {} and size {}", buf.len(), ptr, size
        )));
    }
    Ok(())
}

// Validate both input indexing and the output Vec's byte capacity before allocation.
pub(crate) fn checked_batch_len<T>(input_len: usize, entries: usize, input_channels: usize, output_channels: usize) -> Result<usize> {
    let invalid_size = || Error::new(ErrorKind::InvalidInput, "Batch size overflow");
    let input = entries.checked_mul(input_channels).ok_or_else(invalid_size)?;
    let output = entries.checked_mul(output_channels).ok_or_else(invalid_size)?;
    let bytes = output.checked_mul(std::mem::size_of::<T>()).ok_or_else(invalid_size)?;
    if bytes > isize::MAX as usize {
        return Err(invalid_size());
    }
    if input_len < input {
        return Err(Error::other("Data shotage"));
    }
    Ok(output)
}

pub fn dump(filename:String,buf:&mut [u8]) -> Result<()>  {
    let mut file = File::create(filename)?;
    file.write_all(buf)?;
    file.flush()?;
    Ok(())
}

/// Read an entire file with no input-size limit (legacy compatibility API).
///
/// For untrusted or potentially growing files, use [`load_with_limit`] instead.
/// This header loader does not validate all tag payloads.
pub fn load(filename:String) -> Result<ICCProfile> {
    let mut file = File::open(filename)?;
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;
    ICCProfile::new(&buf)
}

pub fn icc_profile_header_print(header: &ICCProfile) -> String {
    let mut str = "=========== ICC Profile ===========\n".to_string();
    str += &format!("cmmid {}\n",read_ascii_string(&header.cmmid.to_be_bytes(),0,4));
    str += &format!("version {:08x}\n",&header.version);
    str += &format!("Device Class {}\n",read_ascii_string(&header.device_class.to_be_bytes(),0,4));
    str += &format!("Color Space {}\n",read_ascii_string(&header.color_space.to_be_bytes(),0,4));
    str += &format!("PCS {}\n",read_ascii_string(&header.pcs.to_be_bytes(),0,4));
    str += &format!("DATE {}\n",header.create_date);
    str += &format!("It MUST be 'ascp' {}\n",read_ascii_string(&header.magicnumber_ascp.to_be_bytes(),0,4));
    str += &format!("Platform {}\n",read_ascii_string(&header.platform.to_be_bytes(),0,4));
    str += &format!("flags {}\n",&header.flags);
    str += &format!("manufacturer {}\n",read_ascii_string(&header.manufacturer.to_be_bytes(),0,4));
    str += &format!("Model {:04x}\n",&header.model);
    str += &format!("Attributes {:>064b}\n",&header.attributes);
    str += &format!("Illiuminate X:{} Y:{} Z:{}\n",&header.illuminate.x.as_f64(),&header.illuminate.y.as_f64(),&header.illuminate.z.as_f64());
    str += &format!("Creator {}\n",read_ascii_string(&header.creator.to_be_bytes(),0,4));
    str += &format!("Profile ID (MD5 {:016x})\n",&header.profile_id);
    str += &format!("Data length {}bytes\n",&header.length);
    str
}


/// Decode and format a profile without input, decoding, or output resource limits.
///
/// This legacy API inherits the legacy decoder's validation limitations. To bound
/// formatting of already-decoded data, use [`decoded_print_with_limit`]. An output
/// budget does not bound parsing or allocations performed by the decoder.
pub fn icc_profile_print(icc_profile :&ICCProfile,verbose:usize) -> Result<String> {
    let decoded = DecodedICCProfile::new(&icc_profile.data)?;
    decoded_print(&decoded,verbose)
}

/// decoded_print to String ICC Profile data
/// - verbose > 0  very very long information
///
/// Legacy compatibility formatter: output and intermediate `Data::as_string`
/// allocations are unrestricted, and malformed public payloads may panic in that
/// legacy formatter. Use [`decoded_print_with_limit`] for bounded inspection.
/// Tags are sorted lexicographically for deterministic output.

pub fn decoded_print(header :&DecodedICCProfile,verbose:usize) -> Result<String> {
    let mut str = "=========== ICC Profile ===========\n".to_string();
    str += &format!("cmmid {}\n",read_ascii_string(&header.cmmid.to_be_bytes(),0,4));
    str += &format!("version {}.{:02x}\n",&header.version>>24 & 0xff,&header.version>>16 & 0xff);
    str += &format!("Device Class {}\n",read_ascii_string(&header.device_class.to_be_bytes(),0,4));
    str += &format!("Color Space {}\n",read_ascii_string(&header.color_space.to_be_bytes(),0,4));
    str += &format!("PCS {}\n",read_ascii_string(&header.pcs.to_be_bytes(),0,4));
    str += &format!("DATE {}\n",header.create_date);
    str += &format!("It MUST be 'ascp' {}\n",read_ascii_string(&header.magicnumber_ascp.to_be_bytes(),0,4));
    str += &format!("Platform {}\n",read_ascii_string(&header.platform.to_be_bytes(),0,4));
    str += &format!("flags {}\n",&header.flags);
    str += &format!("manufacturer {}\n",read_ascii_string(&header.manufacturer.to_be_bytes(),0,4));
    str += &format!("Model {:04x}\n",&header.model);
    str += &format!("Attributes {:>064b}\n",&header.attributes);

    let x = header.illuminate.x.as_f64();
    let y = header.illuminate.y.as_f64();
    let z = header.illuminate.z.as_f64();
    str += &format!("Illiuminate X:{} Y:{} Z:{}\n",x,y,z);
    str += &format!("Creator {}\n",read_ascii_string(&header.creator.to_be_bytes(),0,4));
    str += &format!("Profile ID (MD5 {:016x})\n",&header.profile_id);
    str += &format!("Data length {}bytes\n",&header.length);
    str += "\n==== ICC Profiles defined data ====\n";
    for (tag_name,val) in sorted_tags(header)? {
        let tag_name = tag_name.to_string();
        str +=  &format!("Tag name {}\n",tag_name);
        match &*tag_name {
            "A2B0" | "A2B1" | "A2B2" | "B2A0" | "B2A1" | "B2A2" => {
                str += "LUT Table\n";
            },
            "chad" => {
                str += "Chromatic adaptation matrix\n";
            },
            "bkpt" => {
                str += "Media Black Point\n";
            },

            "bXYZ" | "gXYZ" | "rXYZ" => {
                str += "rgb XYZ Tag ";
            },
            "bTRC" | "gTRC" | "rTRC"=> { // bTRC
                str += "Color tone reproduction curve\n";
            },
            "desc" => {
                str += "description: ";
            },
            "cprt" => {
                str += "copyright: ";
            },
            "wtpt" => {
                str += "Media white point\n"
            },
            _ => {
            },

        }
        str += &val.as_string(verbose);
        str += "\n";
    }
    Ok(str)
}

/// Read at most `max_bytes` bytes, rejecting a longer stream with `InvalidData`.
///
/// Enforces the limit while reading, without trusting file metadata or allocating
/// the entire budget up front. Reads at most one extra byte to distinguish exact
/// EOF from an oversized stream; no truncated result is returned. A growing or
/// endless reader is stopped at that boundary. Allocation failures in this
/// buffer return `OutOfMemory`. As with `Read`, a blocking reader may still block.
pub fn read_with_limit<R: Read>(mut reader: R, max_bytes: usize) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let remaining = max_bytes - data.len();
        let request = if remaining == 0 {
            1
        } else {
            remaining.min(chunk.len())
        };
        let count = match reader.read(&mut chunk[..request]) {
            Ok(count) => count,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if count > request || count > remaining {
            return Err(ErrorKind::InvalidData.into());
        }
        if count == 0 {
            return Ok(data);
        }
        let needed = data.len() + count; // count <= max_bytes - data.len()
        if needed > data.capacity() {
            let capacity = needed.max(data.capacity().saturating_mul(2)).min(max_bytes);
            data.try_reserve_exact(capacity - data.len())
                .map_err(|_| Error::from(ErrorKind::OutOfMemory))?;
        }
        data.extend_from_slice(&chunk[..count]);
    }
}

/// Load a legacy ICC header after reading no more than `max_bytes` input bytes.
///
/// Oversized files return `InvalidData`, including files that grow during the
/// read. The budget covers the file bytes, not total memory: `ICCProfile::new`
/// copies the bounded buffer and performs its existing header-only validation.
/// Its allocations and any later tag decoding are outside this reader's budget.
/// Use the checked `Profile`/`ParseLimits` API when full parsing limits are needed.
pub fn load_with_limit(filename: String, max_bytes: usize) -> Result<ICCProfile> {
    let data = read_with_limit(File::open(filename)?, max_bytes)?;
    ICCProfile::new(&data)
}

// A deterministic borrowed index. No cloning of potentially large keys/payloads.
fn sorted_tags(header: &DecodedICCProfile) -> Result<Vec<(&String, &Data)>> {
    let mut tags = Vec::new();
    tags.try_reserve_exact(header.tags.len())
        .map_err(|_| Error::from(ErrorKind::OutOfMemory))?;
    tags.extend(header.tags.iter());
    tags.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    Ok(tags)
}

struct BoundedText {
    text: String,
    limit: usize,
    error: Option<ErrorKind>,
}

impl BoundedText {
    fn new(limit: usize) -> Self {
        Self {
            text: String::new(),
            limit,
            error: None,
        }
    }

    fn remaining(&self) -> usize {
        self.limit - self.text.len()
    }

    fn append(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        self.write_fmt(args)
            .map_err(|_| Error::from(self.error.unwrap_or(ErrorKind::Other)))
    }
}

impl fmt::Write for BoundedText {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.error.is_some() {
            return Err(fmt::Error);
        }
        if text.len() > self.remaining() {
            self.error = Some(ErrorKind::InvalidData);
            return Err(fmt::Error);
        }
        let needed = self.text.len() + text.len();
        if needed > self.text.capacity() {
            let capacity = needed
                .max(self.text.capacity().saturating_mul(2))
                .min(self.limit);
            if self
                .text
                .try_reserve_exact(capacity - self.text.len())
                .is_err()
            {
                self.error = Some(ErrorKind::OutOfMemory);
                return Err(fmt::Error);
            }
        }
        self.text.push_str(text);
        Ok(())
    }
}

// A lower bound on Debug output, not an estimate used to accept output. Each
// string byte / vector entry needs at least one output byte. Preflight all
// variable-size fields so Debug never starts scanning an enormous string before
// its first write, nor walks an enormous vector for a tiny output budget. The
// writer below still checks exact UTF-8 byte counts, including punctuation and
// escaping, before every append. Charge before walking nested vectors.
struct InspectionWork(usize);

impl InspectionWork {
    fn charge(&mut self, minimum_bytes: usize) -> Result<()> {
        self.0 = self
            .0
            .checked_sub(minimum_bytes)
            .ok_or(ErrorKind::InvalidData)?;
        Ok(())
    }

    fn localized(&mut self, value: &MultiLocalizedUnicodeType) -> Result<()> {
        self.charge(value.unicode_strings.len())?;
        for text in &value.unicode_strings {
            self.charge(text.lang.len())?;
            self.charge(text.country.len())?;
            self.charge(text.unicode_string.len())?;
        }
        Ok(())
    }

    fn curves(&mut self, curves: &[Curve]) -> Result<()> {
        self.charge(curves.len())?;
        for curve in curves {
            self.charge(match curve {
                Curve::Curve(values) => values.len(),
                Curve::ParametricCurve(value) => value.vals.len(),
            })?;
        }
        Ok(())
    }

    fn data(&mut self, value: &Data) -> Result<()> {
        match value {
            Data::Raw(name, values) => {
                self.charge(name.len())?;
                self.charge(values.len())?;
            }
            Data::ASCII(text) => self.charge(text.len())?,
            Data::Binary(values) | Data::UInt8NumberArray(values) => self.charge(values.len())?,
            Data::UInt16NumberArray(values) | Data::Curve(values) => self.charge(values.len())?,
            Data::UInt32NumberArray(values) => self.charge(values.len())?,
            Data::UInt64NumberArray(values) => self.charge(values.len())?,
            Data::S15Fixed16NumberArray(values) => self.charge(values.len())?,
            Data::U16Fixed16NumberArray(values) => self.charge(values.len())?,
            Data::XYZNumberArray(values) => self.charge(values.len())?,
            Data::ParametricCurve(value) => self.charge(value.vals.len())?,
            Data::FormulaCurve(value) => self.charge(value.vals.len())?,
            Data::SampledCurve(values) => self.charge(values.len())?,
            Data::Lut8(value) => {
                self.charge(value.e_params.len())?;
                self.charge(value.input_table.len())?;
                self.charge(value.clut_values.len())?;
                self.charge(value.output_table.len())?;
            }
            Data::Lut16(value) => {
                self.charge(value.e_params.len())?;
                self.charge(value.input_table.len())?;
                self.charge(value.clut_values.len())?;
                self.charge(value.output_table.len())?;
            }
            Data::Chromaticity(value) => self.charge(value.cie_xy_coordinate_values.len())?,
            Data::MultiLocalizedUnicode(value) => self.localized(value)?,
            Data::CurveSet(value) => {
                self.charge(value.curve_positions.len())?;
                self.charge(value.data.len())?;
            }
            Data::ProfileDescription(values) => {
                self.charge(values.len())?;
                for value in values {
                    self.charge(value.device_attributes.len())?;
                    self.localized(&value.description_device_manufacturer)?;
                    self.localized(&value.description_device_model)?;
                }
            }
            Data::NamedColor(value) => {
                self.charge(value.prefix_color_name.len())?;
                self.charge(value.suffix_color_name.len())?;
                self.charge(value.entries.len())?;
                for entry in &value.entries {
                    self.charge(entry.root_color_name.len())?;
                    self.charge(entry.coordinate.len())?;
                }
            }
            Data::Descriptor(value) => {
                self.charge(value.ascii_string.len())?;
                self.charge(value.lang.len())?;
                self.charge(value.local_string.len())?;
            }
            Data::LutAtoB(value) => {
                self.curves(&value.b_curves)?;
                self.charge(value.matrix.len())?;
                self.curves(&value.m_curves)?;
                self.charge(value.clut.len())?;
                self.curves(&value.a_curves)?;
            }
            Data::LutBtoA(value) => {
                self.curves(&value.b_curves)?;
                self.charge(value.matrix.len())?;
                self.curves(&value.m_curves)?;
                self.charge(value.clut.len())?;
                self.curves(&value.a_curves)?;
            }
            Data::ResponseCurveSet16(value) => {
                self.charge(value.response_curve_structures.len())?;
                for curve in &value.response_curve_structures {
                    self.charge(curve.for_each_channel.len())?;
                    self.charge(curve.patch_with_the_maximum_colorant_value.len())?;
                    self.charge(curve.response_arrays.len())?;
                }
            }
            Data::CrdInfo(values) => {
                self.charge(values.len())?;
                for text in values {
                    self.charge(text.len())?;
                }
            }
            Data::ColorantTable(value) => {
                self.charge(value.colorant_name.len())?;
                self.charge(value.pcs_values.len())?;
            }
            Data::MultiProcessElements(value) => {
                self.charge(value.process_element_positions.len())?;
                self.charge(value.data.len())?;
            }
            Data::OneDimenstionalCurves(value) => {
                self.charge(value.dimensional_curves.len())?;
                for curve in &value.dimensional_curves {
                    self.charge(curve.len())?;
                }
            }
            Data::MatrixElement(value) => self.charge(value.matrix.len())?,
            // Explicit cases: adding a Data variant must also update this audit.
            Data::DataTimeNumber(_)
            | Data::U16Fixed16Number(_)
            | Data::U8Fixed8Number(_)
            | Data::UInt16Number(_)
            | Data::UInt32Number(_)
            | Data::UInt64Number(_)
            | Data::UInt8Number(_)
            | Data::XYZNumber(_)
            | Data::Float32Number(_)
            | Data::PositionNumber(_)
            | Data::S15Fixed16Number(_)
            | Data::Response16Number(_)
            | Data::U1Fixed15Number(_)
            | Data::ViewConditions(_)
            | Data::Measurement(_)
            | Data::None => {}
        }
        Ok(())
    }
}

/// Inspect decoded fields with a strict UTF-8 output byte limit.
///
/// Returns `InvalidData` if the complete output would exceed `max_output_bytes`,
/// never a truncated success. Returns `OutOfMemory` on fallible buffer/index
/// allocation failure. Zero cannot hold the report and returns `InvalidData`.
///
/// This diagnostic format is deliberately different from [`decoded_print`]:
/// header numbers and payloads use streaming derived `Debug`, with all payload
/// fields included. It does not call `Data::as_string`, trust payload counts, or
/// index payload vectors, so inconsistent caller-constructed fields are safe to
/// inspect (not validated). Debug spelling is not a stable serialization format.
/// Tags are sorted and `chad` is labeled without assuming a source white point.
///
/// Large variable fields are size-checked before formatting. The output buffer
/// grows fallibly within the limit; a borrowed tag index is also fallibly
/// allocated and its entry count is bounded by the remaining output budget.
/// These are formatting limits, not parsing limits, an elapsed-time deadline,
/// or a cap on the memory already held by `header` (including HashMap capacity).
/// MClut's private grid-point vector has a fixed 16-entry parser bound.
pub fn decoded_print_with_limit(
    header: &DecodedICCProfile,
    max_output_bytes: usize,
) -> Result<String> {
    let mut out = BoundedText::new(max_output_bytes);
    // Display writes the date directly, without Debug's string escape scan.
    out.append(format_args!(
        "=========== ICC Profile (bounded debug) ===========\n"
    ))?;
    out.append(format_args!("cmmid {:#010x}\nversion {:#010x}\nDevice Class {:#010x}\nColor Space {:#010x}\nPCS {:#010x}\nDATE {}\n",
        header.cmmid, header.version, header.device_class, header.color_space, header.pcs, header.create_date))?;
    out.append(format_args!("magic {:#010x}\nPlatform {:#010x}\nflags {}\nmanufacturer {:#010x}\nModel {:#010x}\nAttributes {:#018x}\nRendering intent {}\n",
        header.magicnumber_ascp, header.platform, header.flags, header.manufacturer, header.model, header.attributes, header.rendering_intent))?;
    out.append(format_args!("Illuminant {:?}\nCreator {:#010x}\nProfile ID {:#034x}\nData length {}bytes\n\n==== ICC Profiles defined data ====\n",
        header.illuminate, header.creator, header.profile_id, header.length))?;

    let mut work = InspectionWork(out.remaining());
    // Every entry emits at least "Tag name \n" (10 bytes). Bound the index before
    // allocation; charge keys before sorting can compare their bytes.
    work.charge(
        header
            .tags
            .len()
            .checked_mul(10)
            .ok_or(ErrorKind::InvalidData)?,
    )?;
    for (name, value) in &header.tags {
        work.charge(name.len())?;
        work.data(value)?;
    }
    for (name, value) in sorted_tags(header)? {
        out.append(format_args!("Tag name {}\n", name))?;
        if name == "chad" {
            out.append(format_args!("Chromatic adaptation matrix\n"))?;
        }
        out.append(format_args!("{:?}\n", value))?;
    }
    Ok(out.text)
}

pub use crate::color_diff::{ciede2000, delta_e76};

#[cfg(test)]
mod boundary_tests {
    use super::{bound_check, checked_batch_len};

    #[test]
    fn invalid_ranges_never_overflow() {
        for (offset, size) in [(usize::MAX, 1), (1, usize::MAX), (usize::MAX, usize::MAX)] {
            assert!(std::panic::catch_unwind(|| bound_check(&[0], offset, size)).unwrap().is_err());
        }
        assert!(bound_check(&[0], 1, 0).is_ok());
        assert!(bound_check(&[], 0, 0).is_ok());
    }

    #[test]
    fn batch_capacity_limits_without_allocation() {
        let max = isize::MAX as usize;
        assert_eq!(checked_batch_len::<u8>(usize::MAX, max / 3, 3, 3).unwrap(), max / 3 * 3);
        assert!(checked_batch_len::<u8>(usize::MAX, max / 3 + 1, 3, 3).is_err());
        assert!(checked_batch_len::<f64>(usize::MAX, max / 24 + 1, 3, 3).is_err());
        assert!(checked_batch_len::<u8>(usize::MAX, usize::MAX / 4 + 1, 3, 4).is_err());
        assert!(checked_batch_len::<u8>(usize::MAX, usize::MAX, 4, 3).is_err());
    }
}


#[cfg(test)]
mod bounded_allocation_tests {
    use super::*;
    use crate::allocation_probe;

    fn profile() -> DecodedICCProfile {
        DecodedICCProfile::new(&vec![0; 132]).unwrap()
    }

    #[test]
    fn huge_payloads_do_not_allocate_intermediate_renderings() {
        for payload in [
            Data::ASCII("x".repeat(1_000_000)),
            Data::Binary(vec![0; 1_000_000]),
            Data::CrdInfo(vec!["\n".repeat(1_000_000)]),
        ] {
            let mut value = profile();
            value.tags.insert("data".into(), payload);
            let (result, _, allocated) =
                allocation_probe::watch(1_000_000, || decoded_print_with_limit(&value, 1024));
            assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidData);
            assert!(
                allocated <= 4096,
                "large intermediate allocation: {allocated}"
            );
        }
    }

    #[test]
    fn input_output_and_sort_index_allocation_failures_are_errors() {
        let input = vec![0; 9000];
        let result = allocation_probe::fail_once(8192, || read_with_limit(&input[..], 9000));
        assert_eq!(result.unwrap_err().kind(), ErrorKind::OutOfMemory);
        assert_eq!(allocation_probe::failure_hits(), 1);
        let mut output = BoundedText::new(200);
        let input = "x".repeat(100);
        let result = allocation_probe::fail_once(100, || output.append(format_args!("{}", input)));
        assert_eq!(result.unwrap_err().kind(), ErrorKind::OutOfMemory);
        assert_eq!(allocation_probe::failure_hits(), 1);
        let mut value = profile();
        value.tags.insert("data".into(), Data::None);
        let result = allocation_probe::fail_once(std::mem::size_of::<(&String, &Data)>(), || {
            sorted_tags(&value)
        });
        assert_eq!(result.unwrap_err().kind(), ErrorKind::OutOfMemory);
        assert_eq!(allocation_probe::failure_hits(), 1);
    }

    #[test]
    fn streaming_debug_stops_at_the_first_failed_write() {
        use std::cell::Cell;
        struct Counted<'a>(&'a Cell<usize>);
        impl fmt::Debug for Counted<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.set(self.0.get() + 1);
                f.write_str("12345")
            }
        }
        let count = Cell::new(0);
        let values: Vec<_> = (0..100_000).map(|_| Counted(&count)).collect();
        let mut output = BoundedText::new(16);
        assert!(output.append(format_args!("{:?}", values)).is_err());
        assert!(count.get() <= 3);
        assert!(output.text.len() <= 16);
        assert!(output.text.capacity() <= 16);
    }
}
