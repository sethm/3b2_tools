#!/usr/bin/env python


# Copyright 2017 Seth J. Morabito <web@loomcom.com>
#
# Permission is hereby granted, free of charge, to any person obtaining a copy of
# this software and associated documentation files (the "Software"), to deal in
# the Software without restriction, including without limitation the rights to
# use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
# of the Software, and to permit persons to whom the Software is furnished to do
# so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.

#
# This is a very simple script to parse UART commands out of CSV files
# exported from Saleae Logic
#

import csv
import sys
from decimal import *

class CommandParser:

    def get_command_name(self, val):
        """Translate a 1-byte command into a name"""

        cmd = val >> 4

        if cmd == 0x0:
            return "Restore"
        elif cmd == 0x1:
            return "Seek"
        elif cmd == 0x2 or val == 0x3:
            return "Step"
        elif cmd == 0x4 or val == 0x5:
            return "Step In"
        elif cmd == 0x6 or val == 0x7:
            return "Step Out"
        elif cmd == 0x8 or val == 0x9:
            return "Read Sector"
        elif cmd == 0xa or val == 0xb:
            return "Write Sector"
        elif cmd == 0xc:
            return "Read Address"
        elif cmd == 0xd:
            return "Force Interrupt"
        elif cmd == 0xe:
            return "Read Track"
        elif cmd == 0xf:
            return "Write Track"
        else:
            return "????"

    def handle_row(self, ts, val):
        """Take one row of CSV data and parse it into human readable form"""

        num = int(val, 16)

        # The number we get is a 16-bit value with:
        #   - Address encoded in bits 0-3
        #   - Data encodedin bits 4-11
        #   - Chip Enable encoded in bit 12
        #   - Write Gate encoded in bit 13
        #   - Read Gate encoded in bit 14
        #   - Interrupt encoded in bit 15


        # We only care about the RISING edge of a CHIP SELECT,
        # and only if READ GATE or WRITE GATE is low at that time.

        skip_line = False

        addr       = num & 0xf
        data       = (num >> 4) & 0xff
        cen_state  = (num >> 12) & 1
        w_state    = (num >> 13) & 1
        r_state    = (num >> 14) & 1
        int_state  = (num >> 15) & 1

        # No clock transition?
        if self.last_r_state == r_state and \
           self.last_w_state == w_state:
            skip_line = True

        # Falling edge?
        if (self.last_r_state == 1 and r_state == 0) or \
           (self.last_w_state == 1 and w_state == 0):
            skip_line = True

        # Rising edge?
        if (self.last_r_state == 0 and r_state == 1) or \
           (self.last_w_state == 0 and w_state == 1):
            skip_line = False

        if cen_state == 1:
	    skip_line = True

        # Special case. Capture the rising edge of an int.
        if int_state == 1 and self.last_int_state == 0:
            print "%s:\tIRQ\t%s\t\tDELTA=%s ms" % \
                (ts, "INTR", ((ts - self.last_cmd) * 1000).normalize())
            skip_line = True

        if skip_line == True:
            self.last_w_state = w_state
            self.last_r_state = r_state
            self.last_int_state = int_state
            return

        if self.last_r_state == 0:
            direction = "READ"
        else:
            direction = "WRITE"

        print "%s:\t%s\t%x\t%02x" % (ts, direction, addr, data)

        self.last_w_state = w_state
        self.last_r_state = r_state
        self.last_int_state = int_state


        # # Now grab some data

        # if is_read:
        #     direction = "READ"
        # elif is_write:
        #     direction = "WRITE"
        # else:
        #     raise "Impossible state"

        # if is_status:
        #     buf = "STATUS"
        # elif is_command:
        #     buf = "COMMAND"
        # elif is_track:
        #     buf = "TRACK"
        # elif is_sec:
        #     buf = "SEC"
        # elif is_data:
        #     buf = "DATA"

        # data = (num & 0xff0) >> 4

        # # Save off some state so we can calculate C/H/S
        # if is_write:
        #     if is_sec:
        #         self.sec = data
        #     elif is_track or is_data:
        #         self.track = data

        # if is_command:
        #     command_name = self.get_command_name(data)

        #     self.last_cmd = Decimal(ts)

        #     # If this is a READ SECTOR, output C/H/S
        #     if data & 0xe0 == 0x80 or data & 0xe0 == 0xa0:
        #         print "%s:\t%s\t%s\t%02x\t%s %d/%d/%d" % \
        #             (ts, direction, buf, data, command_name, \
        #              self.track, ((data >> 1) & 1), self.sec)
        #     else:
        #         print "%s:\t%s\t%s\t%02x\t%s" % (ts, direction, buf, data, command_name)
        # else:
        #     print "%s:\t%s\t%s\t%02x" % (ts, direction, buf, data)


        # self.last_r_state = r_state
        # self.last_w_state = w_state
        # self.last_int_state = int_state

    def main(self, fname):

        self.last_w_state = 1
        self.last_r_state = 1
        self.last_int_state = 1
        self.last_cmd = 0

        with open(fname, 'rb') as csvfile:
            reader = csv.reader(csvfile, delimiter=',', quotechar='\\')
            for row in reader:
                ts = Decimal(row[0])
                val = row[1]
                self.handle_row(ts, val)

if __name__ == '__main__':
    if len(sys.argv) != 2:
        print "Usage: parse_iu_commands.py <file>"
        exit(1)
    CommandParser().main(sys.argv[1])
