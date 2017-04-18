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
# This is a very simple script to parse disk commands out of
# CSV files exported from Saleae Logic
#

import csv
import sys
from decimal import *

class CommandParser:


    def get_command_name(self, val):
        """Translate a 1-byte command into a name"""

        is_aux_cmd = (val & 0xf0 == 0)

        # If this is an aux command, what is it?
        aux_cmd = val & 0x0f

        # If this is a command for a unit, which one?
        unit = val & 0x07

        # What's the command number?
        cmd = (val & 0xf0) >> 4

        is_bufskew = ((val & 0x08) >> 3) == 1

        if is_aux_cmd:
            if aux_cmd == 1:
                return "AUX:RESET"
            elif aux_cmd == 2:
                return "AUX:CLBUF"
            elif aux_cmd == 4:
                return "AUX:HRSQ"
            elif aux_cmd == 8:
                return "AUX:CLCE"
        else:
            if cmd == 1:
                return "Sense Int. Status - %d" % unit
            if cmd == 2:
                return "Specify - %d" % unit
            if cmd == 3:
                return "Sense Unit Status - %d" % unit
            if cmd == 4:
                return "Detect Error - %d" % unit
            if cmd == 5:
                if is_bufskew:
                    return "Recalibrate [B] - %d" % unit
                else:
                    return "Recalibrate - %d" % unit
            if cmd == 6:
                if is_bufskew:
                    return "Seek [B] - %d" % unit
                else:
                    return "Seek - %d" % unit
            if cmd == 7:
                if is_bufskew:
                    return "Format [S] - %d" % unit
                else:
                    return "Format - %d" % unit
            if cmd == 8:
                if is_bufskew:
                    return "Verify ID [S] - %d" % unit
                else:
                    return "Verify ID - %d" % unit
            if cmd == 9:
                if is_bufskew:
                    return "Read ID [S] - %d" % unit
                else:
                    return "Read ID - %d" % unit
            if cmd == 10:
                return "Read Diag. - %d" % unit
            if cmd == 11:
                return "Read Data - %d" % unit
            if cmd == 12:
                return "Check - %d" % unit
            if cmd == 13:
                return "Scan - %d" % unit
            if cmd == 14:
                return "Verify Data - %d" % unit
            if cmd == 15:
                return "Write Data - %d" % unit




    def handle_row(self, ts, val):
        """Take one row of CSV data and parse it into human readable form"""

        num = int(val, 16)

        # The number we get is a 16-bit value with the i/o data encoded in
        # bits 3-10, buffer encoded in bit 2, write encoded in bit 1,
        # and read encoded in bit 0

        # We also only care about the RISING edge of 0x01 or 0x02.
        # Is it a clock transition we don't care about? (i.e., none of the
        # clocks have switched.

        skip_line = False

        r_state    = num & 0x1
        w_state    = (num & 0x2) >> 1
        a0_state   = (num & 0x4) >> 2
        cs_state   = (num & 0x800) >> 11
        is_read    = self.last_r_state == 0
        is_write   = self.last_w_state == 0
        is_command = a0_state == 1 and is_write
        is_status  = a0_state == 1 and is_read
        is_data    = a0_state == 0
        int_state  = (num & 0x1000) >> 12

        # If there's been no change, this was a transition without a clock
        if self.last_r_state == r_state and \
           self.last_w_state == w_state:
            skip_line = True

        # If this is a falling clock, skip it. Just update the last state.
        if (self.last_r_state == 1 and r_state == 0) or \
           (self.last_w_state == 1 and w_state == 0):
            skip_line = True

        # If this is a rising clock edge, we definitely want it
        if (self.last_r_state == 0 and r_state == 1) or \
           (self.last_w_state == 0 and w_state == 1):
            skip_line = False

        # We only care if CS is low
        if cs_state == 1:
	    skip_line = True

        # Special case. Capture the rising edge of an int.
        if int_state == 1 and self.last_int_state == 0:
            print "%s:\t%s\t\t\tDELTA=%s ms" % (ts, "INTR", ((ts - self.last_cmd) * 1000).normalize())
            skip_line = True

        if skip_line == True:
            self.last_w_state = w_state
            self.last_r_state = r_state
            self.last_int_state = int_state
            return

        # Now grab some data

        if is_read:
            direction = "READ"
        elif is_write:
            direction = "WRITE"
        else:
            raise "Impossible state"

        if is_status:
            buf = "STATUS"
        elif is_command:
            buf = "COMMAND"
        elif is_data:
            buf = "DATA"

        data = (num & 0x07f8) >> 3

        if is_command:
            command_name = self.get_command_name(data)
            self.last_cmd = Decimal(ts)
            print "%s:\t%s\t%s\t%02x\t%s" % (ts, direction, buf, data, command_name)
        else:
            print "%s:\t%s\t%s\t%02x" % (ts, direction, buf, data)


        self.last_r_state = r_state
        self.last_w_state = w_state
        self.last_int_state = int_state

    def main(self, fname):

        self.last_r_state = 1
        self.last_w_state = 1
        self.last_int_state = 0
        self.last_cmd = 0

        with open(fname, 'rb') as csvfile:
            reader = csv.reader(csvfile, delimiter=',', quotechar='\\')
            for row in reader:
                ts = Decimal(row[0])
                val = row[1]
                self.handle_row(ts, val)

if __name__ == '__main__':
    if len(sys.argv) != 2:
        print "Usage: parse_commands.py <file>"
        exit(1)
    CommandParser().main(sys.argv[1])
