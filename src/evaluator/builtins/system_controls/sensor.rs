use super::SensorConst;
use crate::evaluator::object::Object;

use windows::{
    core::{GUID},
    Win32::{
        Devices::Sensors::{
            ISensorManager,
            SENSOR_CATEGORY_BIOMETRIC,
            SENSOR_DATA_TYPE_HUMAN_PRESENCE, SENSOR_DATA_TYPE_HUMAN_PROXIMITY_METERS,
            SENSOR_CATEGORY_ELECTRICAL,
            SENSOR_DATA_TYPE_CAPACITANCE_FARAD, SENSOR_DATA_TYPE_RESISTANCE_OHMS, SENSOR_DATA_TYPE_INDUCTANCE_HENRY, SENSOR_DATA_TYPE_CURRENT_AMPS, SENSOR_DATA_TYPE_VOLTAGE_VOLTS, SENSOR_DATA_TYPE_ELECTRICAL_POWER_WATTS,
            SENSOR_CATEGORY_ENVIRONMENTAL,
            SENSOR_DATA_TYPE_TEMPERATURE_CELSIUS, SENSOR_DATA_TYPE_ATMOSPHERIC_PRESSURE_BAR, SENSOR_DATA_TYPE_RELATIVE_HUMIDITY_PERCENT, SENSOR_DATA_TYPE_WIND_DIRECTION_DEGREES_ANTICLOCKWISE, SENSOR_DATA_TYPE_WIND_SPEED_METERS_PER_SECOND,
            SENSOR_CATEGORY_LIGHT,
            SENSOR_DATA_TYPE_LIGHT_LEVEL_LUX, SENSOR_DATA_TYPE_LIGHT_TEMPERATURE_KELVIN,
            SENSOR_CATEGORY_LOCATION,
            SENSOR_DATA_TYPE_ALTITUDE_SEALEVEL_METERS, SENSOR_DATA_TYPE_LATITUDE_DEGREES, SENSOR_DATA_TYPE_LONGITUDE_DEGREES, SENSOR_DATA_TYPE_SPEED_KNOTS,
            SENSOR_CATEGORY_MECHANICAL,
            SENSOR_DATA_TYPE_WEIGHT_KILOGRAMS, SENSOR_DATA_TYPE_FORCE_NEWTONS, SENSOR_DATA_TYPE_ABSOLUTE_PRESSURE_PASCAL, SENSOR_DATA_TYPE_GAUGE_PRESSURE_PASCAL,
            SENSOR_CATEGORY_MOTION,
            SENSOR_DATA_TYPE_ACCELERATION_X_G, SENSOR_DATA_TYPE_ACCELERATION_Y_G, SENSOR_DATA_TYPE_ACCELERATION_Z_G, SENSOR_DATA_TYPE_ANGULAR_ACCELERATION_X_DEGREES_PER_SECOND_SQUARED,SENSOR_DATA_TYPE_ANGULAR_ACCELERATION_Y_DEGREES_PER_SECOND_SQUARED, SENSOR_DATA_TYPE_ANGULAR_ACCELERATION_Z_DEGREES_PER_SECOND_SQUARED, SENSOR_DATA_TYPE_SPEED_METERS_PER_SECOND,
            SENSOR_CATEGORY_ORIENTATION,
            SENSOR_DATA_TYPE_TILT_X_DEGREES, SENSOR_DATA_TYPE_TILT_Y_DEGREES, SENSOR_DATA_TYPE_TILT_Z_DEGREES, SENSOR_DATA_TYPE_DISTANCE_X_METERS, SENSOR_DATA_TYPE_DISTANCE_Y_METERS, SENSOR_DATA_TYPE_DISTANCE_Z_METERS, SENSOR_DATA_TYPE_MAGNETIC_HEADING_MAGNETIC_NORTH_DEGREES, SENSOR_DATA_TYPE_MAGNETIC_HEADING_TRUE_NORTH_DEGREES, SENSOR_DATA_TYPE_MAGNETIC_HEADING_COMPENSATED_MAGNETIC_NORTH_DEGREES, SENSOR_DATA_TYPE_MAGNETIC_HEADING_COMPENSATED_TRUE_NORTH_DEGREES,
            SENSOR_CATEGORY_SCANNER,
            SENSOR_DATA_TYPE_RFID_TAG_40_BIT,
        },
        System::{
            Com::{
                CoCreateInstance,
                CLSCTX_INPROC_SERVER,
                VARENUM, VT_BOOL, VT_R4, VT_R8, VT_UI8, VT_LPWSTR,
                StructuredStorage::{PROPVARIANT, PropVariantClear},
            }
        },
        UI::Shell::PropertiesSystem::PROPERTYKEY,
    }
};

pub struct Sensor {
    manager: Option<ISensorManager>,
    category: GUID,
    prop_key: PROPERTYKEY,
    vt: VARENUM,
}

impl Sensor {
    pub fn new(category: SensorConst) -> Self {
        unsafe {
            let rclsid = GUID::from_u128(0x77a1c827_fcd2_4689_8915_9d613cc5fa3e);
            let manager: Option<ISensorManager> = CoCreateInstance(&rclsid, None, CLSCTX_INPROC_SERVER).ok();

            let (category, prop_key, vt) = match category {
                SensorConst::SNSR_Biometric_HumanPresense => (SENSOR_CATEGORY_BIOMETRIC, SENSOR_DATA_TYPE_HUMAN_PRESENCE, VT_BOOL),
                SensorConst::SNSR_Biometric_HumanProximity => (SENSOR_CATEGORY_BIOMETRIC, SENSOR_DATA_TYPE_HUMAN_PROXIMITY_METERS, VT_R4),
                SensorConst::SNSR_Electrical_Capacitance => (SENSOR_CATEGORY_ELECTRICAL, SENSOR_DATA_TYPE_CAPACITANCE_FARAD, VT_R8),
                SensorConst::SNSR_Electrical_Resistance => (SENSOR_CATEGORY_ELECTRICAL, SENSOR_DATA_TYPE_RESISTANCE_OHMS, VT_R8),
                SensorConst::SNSR_Electrical_Inductance => (SENSOR_CATEGORY_ELECTRICAL, SENSOR_DATA_TYPE_INDUCTANCE_HENRY, VT_R8),
                SensorConst::SNSR_Electrical_Current => (SENSOR_CATEGORY_ELECTRICAL, SENSOR_DATA_TYPE_CURRENT_AMPS, VT_R8),
                SensorConst::SNSR_Electrical_Voltage => (SENSOR_CATEGORY_ELECTRICAL, SENSOR_DATA_TYPE_VOLTAGE_VOLTS, VT_R8),
                SensorConst::SNSR_Electrical_Power => (SENSOR_CATEGORY_ELECTRICAL, SENSOR_DATA_TYPE_ELECTRICAL_POWER_WATTS, VT_R8),
                SensorConst::SNSR_Environmental_Temperature => (SENSOR_CATEGORY_ENVIRONMENTAL, SENSOR_DATA_TYPE_TEMPERATURE_CELSIUS, VT_R4),
                SensorConst::SNSR_Environmental_Pressure => (SENSOR_CATEGORY_ENVIRONMENTAL, SENSOR_DATA_TYPE_ATMOSPHERIC_PRESSURE_BAR, VT_R4),
                SensorConst::SNSR_Environmental_Humidity => (SENSOR_CATEGORY_ENVIRONMENTAL, SENSOR_DATA_TYPE_RELATIVE_HUMIDITY_PERCENT, VT_R4),
                SensorConst::SNSR_Environmental_WindDirection => (SENSOR_CATEGORY_ENVIRONMENTAL, SENSOR_DATA_TYPE_WIND_DIRECTION_DEGREES_ANTICLOCKWISE, VT_R4),
                SensorConst::SNSR_Environmental_WindSpeed => (SENSOR_CATEGORY_ENVIRONMENTAL, SENSOR_DATA_TYPE_WIND_SPEED_METERS_PER_SECOND, VT_R4),
                SensorConst::SNSR_Light_Lux => (SENSOR_CATEGORY_LIGHT, SENSOR_DATA_TYPE_LIGHT_LEVEL_LUX, VT_R4),
                SensorConst::SNSR_Light_Temperature => (SENSOR_CATEGORY_LIGHT, SENSOR_DATA_TYPE_LIGHT_TEMPERATURE_KELVIN, VT_R4),
                SensorConst::SNSR_Mechanical_Force => (SENSOR_CATEGORY_MECHANICAL, SENSOR_DATA_TYPE_FORCE_NEWTONS, VT_R8),
                SensorConst::SNSR_Mechanical_AbsPressure => (SENSOR_CATEGORY_MECHANICAL, SENSOR_DATA_TYPE_ABSOLUTE_PRESSURE_PASCAL, VT_R8),
                SensorConst::SNSR_Mechanical_GaugePressure => (SENSOR_CATEGORY_MECHANICAL, SENSOR_DATA_TYPE_GAUGE_PRESSURE_PASCAL, VT_R8),
                SensorConst::SNSR_Mechanical_Weight => (SENSOR_CATEGORY_MECHANICAL, SENSOR_DATA_TYPE_WEIGHT_KILOGRAMS, VT_R8),
                SensorConst::SNSR_Motion_AccelerationX => (SENSOR_CATEGORY_MOTION, SENSOR_DATA_TYPE_ACCELERATION_X_G, VT_R8),
                SensorConst::SNSR_Motion_AccelerationY => (SENSOR_CATEGORY_MOTION, SENSOR_DATA_TYPE_ACCELERATION_Y_G, VT_R8),
                SensorConst::SNSR_Motion_AccelerationZ => (SENSOR_CATEGORY_MOTION, SENSOR_DATA_TYPE_ACCELERATION_Z_G, VT_R8),
                SensorConst::SNSR_Motion_AngleAccelX => (SENSOR_CATEGORY_MOTION, SENSOR_DATA_TYPE_ANGULAR_ACCELERATION_X_DEGREES_PER_SECOND_SQUARED, VT_R8),
                SensorConst::SNSR_Motion_AngleAccelY => (SENSOR_CATEGORY_MOTION, SENSOR_DATA_TYPE_ANGULAR_ACCELERATION_Y_DEGREES_PER_SECOND_SQUARED, VT_R8),
                SensorConst::SNSR_Motion_AngleAccelZ => (SENSOR_CATEGORY_MOTION, SENSOR_DATA_TYPE_ANGULAR_ACCELERATION_Z_DEGREES_PER_SECOND_SQUARED, VT_R8),
                SensorConst::SNSR_Motion_Speed => (SENSOR_CATEGORY_MOTION, SENSOR_DATA_TYPE_SPEED_METERS_PER_SECOND, VT_R8),
                SensorConst::SNSR_Scanner_RFIDTag => (SENSOR_CATEGORY_SCANNER, SENSOR_DATA_TYPE_RFID_TAG_40_BIT, VT_UI8),
                SensorConst::SNSR_Scanner_BarcodeData => (SENSOR_CATEGORY_SCANNER, PROPERTYKEY::default(), VT_LPWSTR),
                SensorConst::SNSR_Orientation_TiltX => (SENSOR_CATEGORY_ORIENTATION, SENSOR_DATA_TYPE_TILT_X_DEGREES, VT_R4),
                SensorConst::SNSR_Orientation_TiltY => (SENSOR_CATEGORY_ORIENTATION, SENSOR_DATA_TYPE_TILT_Y_DEGREES, VT_R4),
                SensorConst::SNSR_Orientation_TiltZ => (SENSOR_CATEGORY_ORIENTATION, SENSOR_DATA_TYPE_TILT_Z_DEGREES, VT_R4),
                SensorConst::SNSR_Orientation_DistanceX => (SENSOR_CATEGORY_ORIENTATION, SENSOR_DATA_TYPE_DISTANCE_X_METERS, VT_R4),
                SensorConst::SNSR_Orientation_DistanceY => (SENSOR_CATEGORY_ORIENTATION, SENSOR_DATA_TYPE_DISTANCE_Y_METERS, VT_R4),
                SensorConst::SNSR_Orientation_DistanceZ => (SENSOR_CATEGORY_ORIENTATION, SENSOR_DATA_TYPE_DISTANCE_Z_METERS, VT_R4),
                SensorConst::SNSR_Orientation_MagHeading => (SENSOR_CATEGORY_ORIENTATION, SENSOR_DATA_TYPE_MAGNETIC_HEADING_MAGNETIC_NORTH_DEGREES, VT_R8),
                SensorConst::SNSR_Orientation_TrueHeading => (SENSOR_CATEGORY_ORIENTATION, SENSOR_DATA_TYPE_MAGNETIC_HEADING_TRUE_NORTH_DEGREES, VT_R8),
                SensorConst::SNSR_Orientation_CompMagHeading => (SENSOR_CATEGORY_ORIENTATION, SENSOR_DATA_TYPE_MAGNETIC_HEADING_COMPENSATED_MAGNETIC_NORTH_DEGREES, VT_R8),
                SensorConst::SNSR_Orientation_CompTrueHeading => (SENSOR_CATEGORY_ORIENTATION, SENSOR_DATA_TYPE_MAGNETIC_HEADING_COMPENSATED_TRUE_NORTH_DEGREES, VT_R8),
                SensorConst::SNSR_Location_Altitude => (SENSOR_CATEGORY_LOCATION, SENSOR_DATA_TYPE_ALTITUDE_SEALEVEL_METERS, VT_R8),
                SensorConst::SNSR_Location_Latitude => (SENSOR_CATEGORY_LOCATION, SENSOR_DATA_TYPE_LATITUDE_DEGREES, VT_R8),
                SensorConst::SNSR_Location_Longitude => (SENSOR_CATEGORY_LOCATION, SENSOR_DATA_TYPE_LONGITUDE_DEGREES, VT_R8),
                SensorConst::SNSR_Location_Speed => (SENSOR_CATEGORY_LOCATION, SENSOR_DATA_TYPE_SPEED_KNOTS, VT_R8),
            };
            Self { manager, category, prop_key, vt }
        }
    }
    fn get_value(&self) -> Option<PropVariant> {
        unsafe {
            if self.category == SENSOR_CATEGORY_SCANNER && self.vt == VT_LPWSTR {
                None
            } else {
                let manager = self.manager.as_ref()?;
                let collection = manager.GetSensorsByCategory(&self.category);
                let collection = collection.ok()?;
                let sensor = collection.GetAt(0);
                let sensor = sensor.ok()?;
                let report = sensor.GetData();
                let report = report.ok()?;
                report.GetSensorValue(&self.prop_key).ok()
                    .map(|p| PropVariant(p))
            }
        }
    }
    pub fn get_as_object(&self) -> Object {
        if let Some(prop_var) = self.get_value() {
            if prop_var.is(self.vt) {
                match self.vt {
                    VT_BOOL => prop_var.get_bool().into(),
                    VT_R4 => prop_var.get_r4_value().into(),
                    VT_R8 => prop_var.get_r8_value().into(),
                    VT_UI8 => prop_var.get_ui8_value().into(),
                    // VT_LPWSTR => prop_var.get_string().into(),
                    _ => Object::Empty
                }
            } else {
                Object::Empty
            }
        } else {
            Object::Empty
        }
    }
}

struct PropVariant(PROPVARIANT);

impl PropVariant {
    fn is(&self, vt: VARENUM) -> bool {
        unsafe {
            let pv00 = &self.0.Anonymous.Anonymous;
            pv00.vt == vt
        }
    }
    fn get_r4_value(&self) -> f64 {
        unsafe {
            let pv00 = &self.0.Anonymous.Anonymous;
            pv00.Anonymous.fltVal as f64
        }
    }
    fn get_r8_value(&self) -> f64 {
        unsafe {
            let pv00 = &self.0.Anonymous.Anonymous;
            pv00.Anonymous.dblVal
        }
    }
    fn get_ui8_value(&self) -> f64 {
        unsafe {
            let pv00 = &self.0.Anonymous.Anonymous;
            pv00.Anonymous.uhVal as f64
        }
    }
    fn get_bool(&self) -> bool {
        unsafe {
            let pv00 = &self.0.Anonymous.Anonymous;
            pv00.Anonymous.boolVal.as_bool()
        }
    }
    // fn get_string(&self) -> Option<String> {
    //     unsafe {
    //         let pv00 = &self.0.Anonymous.Anonymous;
    //         pv00.Anonymous.pwszVal;
    //         pwstr.to_string().ok()
    //     }
    // }
}
impl Drop for PropVariant {
    fn drop(&mut self) {
        unsafe {
            let _ = PropVariantClear(&mut self.0);
        }
    }
}