// Generated from usr/share/pulsaros-airpods/dbus/org.pulsaros.AirPods.xml.
// Keep in sync: tests/test_extension_contract.py fails when they diverge.
import Gio from 'gi://Gio';

export const BUS_NAME = 'org.pulsaros.AirPods';
export const OBJECT_PATH = '/org/pulsaros/AirPods';

export const INTERFACE_XML = `<node>
  <interface name="org.pulsaros.AirPods">
    <property name="Connected" type="b" access="read"/>
    <property name="Name" type="s" access="read"/>
    <property name="Address" type="s" access="read"/>
    <property name="ModelId" type="s" access="read"/>
    <property name="Model" type="s" access="read"/>
    <property name="IsHeadphones" type="b" access="read"/>
    <property name="HasAdvertisement" type="b" access="read"/>
    <property name="Left" type="i" access="read"/>
    <property name="Right" type="i" access="read"/>
    <property name="Case" type="i" access="read"/>
    <property name="LeftCharging" type="b" access="read"/>
    <property name="RightCharging" type="b" access="read"/>
    <property name="CaseCharging" type="b" access="read"/>
    <property name="LeftInEar" type="b" access="read"/>
    <property name="RightInEar" type="b" access="read"/>
    <property name="BothPodsInCase" type="b" access="read"/>
    <property name="CaseLidOpened" type="b" access="read"/>
    <property name="FallbackBattery" type="i" access="read"/>
    <property name="EarDetection" type="b" access="readwrite"/>
    <property name="LidPopup" type="b" access="readwrite"/>
    <signal name="LidOpened"/>
  </interface>
</node>`;

export const AirPodsProxy = Gio.DBusProxy.makeProxyWrapper(INTERFACE_XML);
