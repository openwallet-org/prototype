use core::array::TryFromSliceError;

use bip39::ErrorKind;
use heapless::{consts::*, String};
use usb_device::UsbError;

pub type ErrStringType = String<U80>;
pub enum WalletErr {
    NoMsg,
    StringErr(ErrStringType),
}

// impl From<stm32f1xx_hal::flash::Error> for WalletErr {
//     fn from(e: stm32f1xx_hal::flash::Error) -> WalletErr {
//         use stm32f1xx_hal::flash::Error::*;
//         match e {
//             AddressLargerThanFlash => WalletErr::from("FlashError: AddressLargerThanFlash"),
//             AddressMisaligned => WalletErr::from("FlashError: AddressMisaligned"),
//             LengthNotMultiple2 => WalletErr::from("FlashError: LengthNotMultiple2"),
//             LengthTooLong => WalletErr::from("FlashError: LengthTooLong"),
//             EraseError => WalletErr::from("FlashError: EraseError"),
//             ProgrammingError => WalletErr::from("FlashError: ProgrammingError"),
//             WriteError => WalletErr::from("FlashError: WriteError"),
//             VerifyError => WalletErr::from("FlashError: VerifyError"),
//             // VerifyErrorWithVals(vals) => {
//             //     let mut err_str = ErrStringType::from("FlashError: VerifyErrorWithVals: ");
//             //     let mut buf = [0u8; 10];
//             //     let _ = err_str.push_str(vals.0.numtoa_str(16, &mut buf));
//             //     let _ = err_str.push_str(" ");
//             //     let _ = err_str.push_str(vals.1.numtoa_str(16, &mut buf));
//             //     WalletErr::StringErr(err_str)
//             // }
//             UnlockError => WalletErr::from("FlashError: UnlockError"),
//             UnlockOptError => WalletErr::from("FlashError: UnlockOptError"),
//             LockError => WalletErr::from("FlashError: LockError"),
//         }
//     }
// }

impl From<TryFromSliceError> for WalletErr {
    fn from(_: TryFromSliceError) -> WalletErr {
        WalletErr::from("try from slice failed")
    }
}

impl From<stm32f4xx_hal::flash::Error> for WalletErr {
    fn from(e: stm32f4xx_hal::flash::Error) -> WalletErr {
        match e {
            stm32f4xx_hal::flash::Error::ProgrammingSequence => {
                WalletErr::from("ProgrammingSequence")
            }
            stm32f4xx_hal::flash::Error::ProgrammingParallelism => {
                WalletErr::from("ProgrammingParallelism")
            }
            stm32f4xx_hal::flash::Error::ProgrammingAlignment => {
                WalletErr::from("ProgrammingAlignment")
            }
            stm32f4xx_hal::flash::Error::WriteProtection => WalletErr::from("WriteProtection"),
            stm32f4xx_hal::flash::Error::Operation => WalletErr::from("Operation"),
        }
    }
}

impl From<aes_ccm::Error> for WalletErr {
    fn from(e: aes_ccm::Error) -> WalletErr {
        match e {
            aes_ccm::Error => WalletErr::from("AES error"),
        }
    }
}

impl From<UsbError> for WalletErr {
    fn from(e: usb_device::UsbError) -> WalletErr {
        use usb_device::UsbError::*;
        match e {
            WouldBlock => WalletErr::from("UsbError: An operation would block because the device is currently busy or there is no data available."),
            ParseError => WalletErr::from("UsbError: Parsing failed due to invalid input.,"),
            BufferOverflow => WalletErr::from("UsbError: A buffer too short for the data to read was passed, or provided data cannot fit within length constraints."),
            EndpointOverflow => WalletErr::from("UsbError: Classes attempted to allocate more endpoints than the peripheral supports."),
            EndpointMemoryOverflow => WalletErr::from("UsbError: Classes attempted to allocate more packet buffer memory than the peripheral supports."),
            InvalidEndpoint => WalletErr::from("UsbError: The endpoint address is invalid or already used."),
            Unsupported => WalletErr::from("UsbError: Operation is not supported by device or configuration."),
            InvalidState => WalletErr::from("UsbError: Operation is not valid in the current state of the object."),
        }
    }
}

impl From<postcard::Error> for WalletErr {
    fn from(e: postcard::Error) -> WalletErr {
        use postcard::Error::*;
        match e {
            WontImplement => WalletErr::from("PostcardError: This is a feature that PostCard will never implement"),
            NotYetImplemented => WalletErr::from("PostcardError: This is a feature that Postcard intends to support, but does not yet"),
            SerializeBufferFull => WalletErr::from("PostcardError: The serialize buffer is full"),
            SerializeSeqLengthUnknown => WalletErr::from("PostcardError: The length of a sequence must be known"),
            DeserializeUnexpectedEnd => WalletErr::from("PostcardError: Hit the end of buffer, expected more data"),
            DeserializeBadVarint => WalletErr::from("PostcardError: Found a varint that didn't terminate. Is the usize too big for this platform?"),
            DeserializeBadBool => WalletErr::from("PostcardError: Found a bool that wasn't 0 or 1"),
            DeserializeBadChar => WalletErr::from("PostcardError: Found an invalid unicode char"),
            DeserializeBadUtf8 => WalletErr::from("PostcardError: Tried to parse invalid utf-8"),
            DeserializeBadOption => WalletErr::from("PostcardError: Found an Option discriminant that wasn't 0 or 1"),
            DeserializeBadEnum => WalletErr::from("PostcardError: Found an enum discriminant that was > u32::max_value()"),
            DeserializeBadEncoding => WalletErr::from("PostcardError: The original data was not well encoded"),
            SerdeSerCustom => WalletErr::from("PostcardError: Serde Serialization Error"),
            SerdeDeCustom => WalletErr::from("PostcardError: Serde Deserialization Error"),
        }
    }
}

impl From<k256::ecdsa::Error> for WalletErr {
    fn from(_: k256::ecdsa::Error) -> WalletErr {
        WalletErr::from("ECDSA error")
    }
}

impl From<&'static str> for WalletErr {
    fn from(s: &'static str) -> WalletErr {
        WalletErr::StringErr(ErrStringType::from(s))
    }
}

impl From<ErrorKind> for WalletErr {
    fn from(e: ErrorKind) -> WalletErr {
        match e {
            ErrorKind::InvalidChecksum => WalletErr::from("BIP39: invalid_checksum"),
            ErrorKind::InvalidWord(_) => WalletErr::from("BIP39: invalid_word"),
            ErrorKind::InvalidKeysize(_) => WalletErr::from("BIP39: invalid_keysize"),
            ErrorKind::InvalidWordLength(_) => WalletErr::from("BIP39: invalid_word_length"),
            ErrorKind::InvalidEntropyLength(_, _) => {
                WalletErr::from("BIP39: invalid_entropy_length")
            }
        }
    }
}

impl From<tiny_hderive::Error> for WalletErr {
    fn from(e: tiny_hderive::Error) -> WalletErr {
        match e {
            tiny_hderive::Error::Secp256k1 => WalletErr::from("Secp256k1"),
            tiny_hderive::Error::InvalidChildNumber => WalletErr::from("InvalidChildNumber"),
            tiny_hderive::Error::InvalidDerivationPath => WalletErr::from("InvalidDerivationPath"),
            tiny_hderive::Error::InvalidExtendedPrivKey => {
                WalletErr::from("InvalidExtendedPrivKey")
            }
        }
    }
}
