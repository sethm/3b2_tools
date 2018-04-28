#include <iostream>
#include <sstream>
#include <bitset>
#include <cstdint>

using std::cerr;
using std::cout;
using std::endl;

void output_exec_level(uint16_t level) {
    cout << "\t(";
    switch (level) {
    case 0:
        cout << "Kernel";
        break;
    case 1:
        cout << "Executive";
        break;
    case 2:
        cout << "Supervisor";
        break;
    case 3:
        cout << "User";
        break;
    default:
        cerr << "UNEXPECTED PM!" << endl;
        return;
    }
    cout << ")" << endl;
}

void translate_psw(uint32_t psw) {
    cout << "PSW: 0x" << std::hex << psw << endl;

    cout << endl;

    uint16_t et = psw & 0x03;
    uint16_t tm = psw >> 2 & 0x01;
    uint16_t isc = psw >> 3 & 0x0f;
    uint16_t i = psw >> 7 & 0x01;
    uint16_t r = psw >> 8 & 0x01;
    uint16_t pm = psw >> 9 & 0x03;
    uint16_t cm = psw >> 11 & 0x03;
    uint16_t ipl = psw >> 13 & 0x0f;
    uint16_t te = psw >> 17 & 0x01;
    uint16_t c = psw >> 18 & 0x01;
    uint16_t v = psw >> 19 & 0x01;
    uint16_t z = psw >> 20 & 0x01;
    uint16_t n = psw >> 21 & 0x01;
    uint16_t oe = psw >> 22 & 0x01;
    uint16_t cd = psw >> 23 & 0x01;
    uint16_t qie = psw >> 24 & 0x01;
    uint16_t cfd = psw >> 25 & 0x01;

    cout << "ET:\t" << et << "\t(";
    switch(et) {
    case 0:
        cout << "On Reset Exception";
        break;
    case 1:
        cout << "On Process Exception";
        break;
    case 2:
        cout << "On Stack Exception";
        break;
    case 3:
        cout << "On Normal Exception";
        break;
    default:
        cerr << "UNEXPECTED ET!" << endl;
        return;
    }
    cout << ")" << endl;

    cout << "TM:\t" << tm << endl;
    cout << "ISC:\t" << std::bitset<4>(isc) << "b" << endl;
    cout << "I:\t" << i << endl;
    cout << "R:\t" << r << endl;

    cout << "PM:\t" << pm;
    output_exec_level(pm);

    cout << "CM:\t" << cm;
    output_exec_level(cm);

    cout << "IPL:\t" << std::bitset<4>(ipl) << "b" << endl;

    cout << "TE:\t" << te << endl;
    cout << "C Flag:\t" << c << endl;
    cout << "V Flag:\t" << v << endl;
    cout << "Z Flag:\t" << z << endl;
    cout << "N Flag:\t" << n << endl;
    cout << "OE:\t" << oe << endl;
    cout << "CD:\t" << cd << endl;
    cout << "QIE:\t" << qie << endl;
    cout << "CFD:\t" << cfd << endl;

}

int main(int argc, char** argv) {
    std::stringstream hex_stream;
    uint32_t psw;

    if (argc != 2) {
        cerr << "usage: psw <status word>" << endl;
        return 1;
    }

    hex_stream << std::hex << argv[1];
    hex_stream >> psw;

    translate_psw(psw);

    return 0;
}
