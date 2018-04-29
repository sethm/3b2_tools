#!/usr/bin/env python3

from __future__ import print_function
import sys, re
from collections import namedtuple

# Each entry has:
#   - Number of arguments
#   - Name of routine
syscalls = [
    (0, 0, 'nosys'),
    (1, 1, 'rexit'),
    (2, 0, 'fork'),
    (3, 3, 'read'),
    (4, 3, 'write'),
    (5, 3, 'open'),
    (6, 1, 'close'),
    (7, 0, 'wait'),
    (8, 2, 'creat'),
    (9, 2, 'link'),
    (0, 1, 'unlink'),
    (11, 2, 'exec'),
    (12, 1, 'chdir'),
    (13, 0, 'gtime'),
    (14, 3, 'mknod'),
    (15, 2, 'chmod'),
    (16, 3, 'chown'),
    (17, 1, 'sbreak'),
    (18, 2, 'stat'),
    (19, 3, 'seek'),
    (20, 0, 'getpid'),
    (21, 4, 'smount'),
    (22, 1, 'sumount'),
    (23, 1, 'setuid'),
    (24, 0, 'getuid'),
    (25, 1, 'stime'),
    (26, 4, 'ptrace'),
    (27, 1, 'alarm'),
    (28, 2, 'fstat'),
    (29, 0, 'pause'),
    (30, 2, 'utime'),
    (31, 2, 'stty'),
    (32, 2, 'gtty'),
    (33, 2, 'saccess'),
    (34, 1, 'nice'),
    (35, 4, 'statfs'),
    (36, 0, 'sync'),
    (37, 2, 'kill'),
    (38, 4, 'fstatfs'),
    (39, 1, 'setpgrp'),
    (40, 0, 'nosys'),
    (41, 1, 'dup'),
    (42, 0, 'pipe'),
    (43, 1, 'times'),
    (44, 4, 'profil'),
    (45, 1, 'lock'),
    (46, 1, 'setgid'),
    (47, 0, 'getgid'),
    (48, 2, 'ssig'),
    (49, 6, 'msgsys'),
    (50, 4, 'sys3b'),
    (51, 1, 'sysacct'),
    (52, 4, 'shmsys'),
    (53, 5, 'semsys'),
    (54, 3, 'ioctl'),
    (55, 3, 'uadmin'),
    (56, 0, 'nosys'),
    (57, 3, 'utssys'),
    (58, 0, 'nosys'),
    (59, 3, 'exece'),
    (60, 1, 'umask'),
    (61, 1, 'chroot'),
    (62, 3, 'fcntl'),
    (63, 2, 'ulimit'),
    (64, 0, 'nosys'),
    (65, 0, 'nosys'),
    (66, 0, 'nosys'),
    (67, 0, 'nosys'),
    (68, 0, 'nosys'),
    (69, 0, 'nosys'),
    (70, 4, 'advfs'),
    (71, 1, 'unadvfs'),
    (72, 4, 'rmount'),
    (73, 1, 'rumount'),
    (74, 5, 'rfstart'),
    (75, 0, 'nosys'),
    (76, 1, 'rdebug'),
    (77, 0, 'rfstop'),
    (78, 6, 'rfsys'),
    (79, 1, 'rmdir'),
    (80, 2, 'mkdir'),
    (81, 4, 'getdents'),
    (82, 0, 'nosys'),
    (83, 0, 'nosys'),
    (84, 3, 'sysfs'),
    (85, 4, 'getmsg'),
    (86, 4, 'putmsg'),
    (87, 3, 'poll')
]

#
# A SystemCall represents an individual call
#
SystemCall = namedtuple('SystemCall', \
                        ['line_num', 'addr', 'call_num', \
                         'call_name', 'argc', 'args'])


def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)

class SyscallFinder:

    def __init__(self, infile):
        self.pattern = re.compile(r'GATE')
        self.addr_pattern = re.compile(r'^[a-f0-9]{8} ([a-f0-9]{8})\|')
        self.infile = infile
        with open(self.infile, 'r') as f:
            self.lines = f.readlines()

    def get_syscall(self, gate_line):
        # The line previous to the GATE will contain the
        # syscall number.
        addr = re.match(self.addr_pattern, self.lines[gate_line]).group(1)
        hexnum = re.split(":? +", self.lines[gate_line - 1])[1]
        call_num = int(int(hexnum, 16) / 8)
        argc = syscalls[call_num][1]
        call_name = syscalls[call_num][2]

        return SystemCall(gate_line, addr, call_num, call_name, argc, [])


    def handle_syscall(self, gate_line):
        system_call = self.get_syscall(gate_line)

        print(system_call)
        # for j in range(30, 3, -1):
        #     sys.stdout.write(self.lines[gate_line - j])

    def find_syscalls(self):
        for i in range(len(self.lines)):
            if self.pattern.search(self.lines[i]):
                self.handle_syscall(i)

if __name__ == "__main__":
    if len(sys.argv) != 2:
        eprint('usage: syscalls <log_file>');
        exit(1)

    infile = sys.argv[1]
    parser = SyscallFinder(infile)
    parser.find_syscalls()
