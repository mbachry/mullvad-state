#!/usr/bin/env python3
from enum import Enum

import dbus


class ConnectionState(Enum):
    Connected = 1
    Disconnected = 2
    Unknown = 3
    Uninitialized = 4


class Py3status:
    cache_timeout = 1
    icon_on = '●'
    icon_off = '■'
    icon_unknown = 'X'
    format = '{icon}'

    def _get_state(self):
        bus = dbus.SessionBus()
        proxy = bus.get_object('org.mbachry.Mullvad', '/org/mbachry/Mullvad')
        iface = dbus.Interface(proxy, 'org.mbachry.Mullvad')
        value = iface.GetVpnState()
        return ConnectionState[value]

    # Method run by py3status
    def mullvad(self):
        color = self.py3.COLOR_BAD

        try:
            state = self._get_state()
        except Exception:
            icon = self.icon_unknown
            color = self.py3.COLOR_BAD
        else:
            if state == ConnectionState.Connected:
                icon = self.icon_on
                color = self.py3.COLOR_GOOD
            elif state == ConnectionState.Disconnected:
                icon = self.icon_off
                color = self.py3.COLOR_BAD
            elif state == ConnectionState.Unknown:
                icon = self.icon_unknown
                color = self.py3.COLOR_BAD
            else:
                raise ValueError(f'unknow state: {state}')

        full_text = self.py3.safe_format(self.format, {'icon': icon})
        return {
            'full_text': full_text,
            'color': color,
            'cached_until': self.py3.time_in(self.cache_timeout),
        }


if __name__ == "__main__":
    from py3status.module_test import module_test

    module_test(Py3status)
