use anyhow::anyhow;
use std::num::{IntErrorKind, NonZeroI32};
use std::sync::Once;

use embedded_storage::{ReadStorage, Storage};
use esp_storage::FlashStorage;
use rand::random;

use esp_idf_svc::hal::spi::SpiDriver;
use esp_idf_svc::sys::EspError;

use crate::{Pn532, Pn532Error};

mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;

/// PN532 response buffer size. Must be big enough to hold any expected responses.
const PN532_BUF_SIZE: usize = 128;

/// Start of NVS partition.
const FLASH_ADDR: u32 = 0x9000;

const YUBIKEY_AID: [u8; 7] = [0xA0, 0x00, 0x00, 0x05, 0x27, 0x20, 0x01];

static mut FLASH: Option<FlashStorage> = None;

static mut PN532: Option<Pn532<SpiDriver, PN532_BUF_SIZE>> = None;

static INIT_FLASH: Once = Once::new();
static INIT_PN532: Once = Once::new();

/// Prints debug messages from C code.
///
/// # Safety
///
/// Undefined behavior may occur when `message` is passed to `std::ffi::CStr::from_ptr`.
///
/// - The memory pointed to by `message` must contain a valid nul terminator at the end of the string.
/// - `message` must be valid for reads of bytes up to and including the nul terminator. This means in particular:
///   - The entire memory range of this `CStr` must be contained within a single allocated object!
///   - `message` must be non-null even for a zero-length cstr.
//  - The nul terminator must be within isize::MAX from `message`.
#[no_mangle]
pub unsafe extern "C" fn ykhmac_debug_print(message: *const ::core::ffi::c_char) {
    // Convert the raw pointer to a CStr
    let c_str: &std::ffi::CStr = unsafe { std::ffi::CStr::from_ptr(message) };
    // Convert the CStr to a &str
    let str_slice: &str = c_str.to_str().expect("Failed to convert CStr to str");
    print!("{}", str_slice)
}

/// Returns a random `u8`.
#[no_mangle]
pub extern "C" fn ykhmac_random() -> u8 {
    random::<u8>()
}

/// Performs the `InDataExchange` command with the PN532. `send_buffer` is sent and
/// `response_length` bytes of the response will be loaded into `response_buffer`.
/// If the actual response is shorter than `response_length`, the value of `response_length` will be updated.
///
/// # Safety
///
/// This function dereferences the raw pointer to `send_buffer`, `response_buffer`,
/// and `response_length` after confirming they are not `null`.
///
/// The same precautions as `std::slice::from_raw_parts_mut` should be taken to avoid
/// undefined behavior for `send_buffer` and `response_buffer`.
#[no_mangle]
pub unsafe extern "C" fn ykhmac_data_exchange(
    send_buffer: *mut u8,
    send_length: u8,
    response_buffer: *mut u8,
    response_length: *mut u8,
) -> bool {
    if send_buffer.is_null() || response_buffer.is_null() || response_length.is_null() {
        log::error!("One or more inputs for data exchange are null");
        return false;
    }

    let pn532 = match get_pn532() {
        Ok(device) => device,
        Err(e) => {
            log::error!("Cannot get PN532: {e:?}");
            return false;
        }
    };

    let send_bytes: &mut [u8] =
        unsafe { std::slice::from_raw_parts_mut(send_buffer, send_length as usize) };
    let response_bytes: &mut [u8] =
        unsafe { std::slice::from_raw_parts_mut(response_buffer, *response_length as usize) };
    unsafe {
        match pn532.in_data_exchange(send_bytes, response_bytes) {
            Ok(actual_length) => {
                *response_length = actual_length;
            }
            Err(_) => return false,
        }
    }
    true
}

/// Writes data from the `data` buffer into persistent memory.
///
/// # Safety
///
/// This function dereferences the raw pointer to `data` after confirming it is not `null`.
/// The same precautions as `std::slice::from_raw_parts` should be taken to avoid undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn ykhmac_presistent_write(
    data: *const u8,
    size: usize,
    offset: usize,
) -> bool {
    if data.is_null() || size == 0 {
        log::error!("Persistent write data is null or size is 0");
        return false;
    }
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(data, size) };
    let offset = offset as u32;

    let flash = get_flash();

    if let Err(e) = flash.write(FLASH_ADDR + offset, bytes) {
        log::error!("Failed to write to flash storage: {e:?}");
        return false;
    }
    log::info!("Written to 0x{:X}: {:02X?}", FLASH_ADDR + offset, bytes);

    // Read-back test
    let mut reread_bytes = [0u8; EEPROM_SIZE as usize];
    if let Err(e) = flash.read(FLASH_ADDR + offset, &mut reread_bytes[..size]) {
        log::error!("Failed to read from flash storage: {e:?}");
        return false;
    }
    log::info!(
        "Read-back from 0x{:X}:  {:02X?}",
        FLASH_ADDR + offset,
        &reread_bytes[..size]
    );
    if &reread_bytes[..size] != bytes {
        log::error!("Flash storage read-back test failed");
        return false;
    }
    true
}

/// Reads data from persistent memory into the `data` buffer.
///
/// # Safety
///
/// This function dereferences the raw pointer to `data` after confirming it is not `null`.
/// The same precautions as `std::slice::from_raw_parts_mut` should be taken to avoid undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn ykhmac_presistent_read(data: *mut u8, size: usize, offset: usize) -> bool {
    if data.is_null() || size == 0 {
        log::error!("Persistent read buffer is null or size is 0");
        return false;
    }
    let bytes: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(data, size) };
    let offset = offset as u32;

    let flash = get_flash();

    if let Err(e) = flash.read(FLASH_ADDR + offset, &mut bytes[..size]) {
        log::error!("Failed to read from flash storage: {e:?}");
        return false;
    }
    log::info!(
        "Read from 0x{:X}:  {:02X?}",
        FLASH_ADDR + offset,
        &bytes[..size]
    );
    true
}

/// Obtains a mutable reference to the shared FlashStorage instance.
/// Initializes the shared FlashStorage instance on first call.
fn get_flash() -> &'static mut FlashStorage {
    // Use the `Once` pattern to ensure the FlashStorage is initialized only once
    INIT_FLASH.call_once(|| unsafe {
        FLASH = Some(FlashStorage::new());
        log::info!(
            "Initialized Flash Storage. Size = {} bytes",
            FLASH.as_mut().unwrap().capacity()
        );
    });

    unsafe {
        FLASH
            .as_mut()
            .expect("Cannot obtain reference to FlashStorage instance")
    }
}

/// Initializes the shared PN532 instance.
pub fn initialize_pn532(
    mut pn532: Pn532<'static, SpiDriver<'static>, PN532_BUF_SIZE>,
) -> anyhow::Result<(), Pn532Error> {
    INIT_PN532.call_once(|| {
        log::info!("Initialized PN532");
        if let Err(e) = pn532.print_firmware_version() {
            log::error!("Cannot get firmware version! {e:?}");
            return;
        };
        if let Err(e) = pn532.sam_config() {
            log::error!("Cannot set SAM config! {e:?}");
            return;
        };
        if let Err(e) = pn532.set_passive_activation_retries(0xFF) {
            log::error!("Cannot set retries! {e:?}");
            return;
        };
        unsafe {
            PN532 = Some(pn532);
        }
    });
    unsafe {
        if PN532.is_none() {
            return Err(Pn532Error::InterfaceError(EspError::from_non_zero(
                NonZeroI32::try_from(0x103).expect("Unable to convert EspError code"),
            )));
        }
    }
    Ok(())
}

/// Obtains a mutable reference to the shared PN532 instance.
pub fn get_pn532<'d>(
) -> anyhow::Result<&'static mut Pn532<'d, SpiDriver<'d>, PN532_BUF_SIZE>, Pn532Error> {
    unsafe {
        if PN532.is_none() {
            log::error!("PN532 not initialized, cannot get reference");
            return Err(Pn532Error::InterfaceError(EspError::from_non_zero(
                NonZeroI32::try_from(0x103).expect("Unable to convert EspError code"),
            )));
        }
        Ok(PN532
            .as_mut()
            .expect("Cannot obtain reference to Pn532 instance"))
    }
}

/// Enrolls a secret key into encrypted persistent memory.
pub fn enroll_key(hex_str: &str) -> anyhow::Result<()> {
    let mut secret_key = [0u8; SECRET_KEY_SIZE as usize];
    if let Err(e) = input_secret_key(hex_str, &mut secret_key) {
        return Err(anyhow!("{:?}", e));
    }
    log::info!("Secret key: {secret_key:02X?}");
    unsafe {
        if !ykhmac_enroll_key(secret_key.as_mut_ptr()) {
            log::error!("Failed to enroll key");
            Err(anyhow!("Failed to enroll key"))
        } else {
            Ok(())
        }
    }
}

/// Converts each chunk of 2 in the given hex string into a `u8` and fills them into `buf`.
fn input_secret_key(
    hex_str: &str,
    buf: &mut [u8; SECRET_KEY_SIZE as usize],
) -> anyhow::Result<(), IntErrorKind> {
    if !is_hex_string(hex_str) {
        return Err(IntErrorKind::InvalidDigit);
    }

    buf.fill(0); // Pad with zeros if secret key is shorter than `SECRET_KEY_SIZE`.
    let mut hex = hex_str
        .as_bytes()
        .chunks(2) // Each 2 hex chars are treated as 1 `u8` (e.g. "FF" -> 0xFF)
        .map(|chunk| {
            let substr: String = chunk.iter().map(|&c| c as char).collect();
            u8::from_str_radix(&substr, 16).expect("Invalid hex character in secret key")
        })
        .collect::<Vec<u8>>();

    if hex.len() > SECRET_KEY_SIZE as usize {
        log::warn!(
            "Secret key too long, truncating to {} characters",
            SECRET_KEY_SIZE * 2
        )
    }
    hex.resize_with(SECRET_KEY_SIZE as usize, Default::default);
    buf.clone_from_slice(&hex[..SECRET_KEY_SIZE as usize]);
    Ok(())
}

/// Returns `true` if each character in the string is a hexadecimal digit.
fn is_hex_string(input: &str) -> bool {
    input.chars().all(|c| c.is_ascii_hexdigit())
}

/// Waits for a YubiKey and then performs challenge-response.
/// Returns `true` on successful authentication.
pub fn authenticate() -> bool {
    let pn532 = match get_pn532() {
        Ok(device) => device,
        Err(e) => {
            log::error!("Cannot get PN532: {e:?}");
            return false;
        }
    };

    if pn532.inlist_passive_target().is_ok() {
        unsafe {
            if ykhmac_select(YUBIKEY_AID.as_ptr(), 7) {
                log::info!("Select OK");
                return if ykhmac_authenticate(SLOT_2 as u8) {
                    log::info!("Access granted :)");
                    true
                } else {
                    log::info!("Communication error or access denied :(");
                    false
                };
            }
        }
    }
    false
}
