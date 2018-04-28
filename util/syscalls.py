#!/usr/bin/env python

import sys
import re

calls = [
    'nosys',
    'rexit',
    'fork',
    'read',
    'write',
    'open',
    'close',
    'wait',
    'creat',
    'link',
    'unlink',
    'exec',
    'chdir',
    'gtime',
    'mknod',
    'chmod',
    'chown',
    'sbreak',
    'stat',
    'seek',
    'getpid',
    'smount',
    'sumount',
    'setuid',
    'getuid',
    'stime',
    'ptrace',
    'alarm',
    'fstat',
    'pause',
    'utime',
    'stty',
    'gtty',
    'saccess',
    'nice',
    'statfs',
    'sync',
    'kill',
    'fstatfs',
    'setpgrp',
    'nosys',
    'dup',
    'pipe',
    'times',
    'profil',
    'lock',
    'setgid',
    'getgid',
    'ssig',
    'msgsys',
    'sys3b',
    'sysacct',
    'shmsys',
    'semsys',
    'ioctl',
    'uadmin',
    'nosys',
    'utssys',
    'nosys',
    'exece',
    'umask',
    'chroot',
    'fcntl',
    'ulimit',
    'nosys',
    'nosys',
    'nosys',
    'nosys',
    'nosys',
    'nosys',
    'advfs',
    'unadvfs',
    'rmount',
    'rumount',
    'rfstart',
    'nosys',
    'rdebug',
    'rfstop',
    'rfsys',
    'rmdir',
    'mkdir',
    'getdents',
    'libattach',
    'libdetach',
    'sysfs',
    'getmsg',
    'putmsg',
    'poll'
]



class SyscallParser:

    def handle_line(self, line):
        if re.match('.*MOVW\s+&0x4,%r0', line) or \
           re.match('.*MOVW.*,%r1', line):
            self.in_syscall = True

        m = re.match('.*MOVW\s+&0x(.*),%r1', line)
        if m:
            self.syscall_number = int(m.group(1), 16) / 8

        sys.stdout.write(line)
            
        if re.match('.*GATE', line) and self.in_syscall:
            if self.syscall_number >= 0 and self.syscall_number < len(calls):
                print('    <Syscall %s>' % calls[self.syscall_number])
            else:
                print('    <Syscall %d>' % (self.syscall_number))
        else:
            print

        if not re.match('.*MOVW.*&0x4,%r0', line) and \
           not re.match('.*MOVW.*,%r1', line):
            self.in_syscall = False


    def main(self, fname):
        self.in_syscall = False

        with open(fname, 'rb') as logfile:
            for l in logfile:
                l = l.strip()
                self.handle_line(l)


if __name__ == '__main__':
    if len(sys.argv) != 2:
        print("Usage: syscalls <logfile>")
        exit(1)

    SyscallParser().main(sys.argv[1])
