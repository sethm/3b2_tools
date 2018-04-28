#!/usr/bin/env ruby
#
# Copyright (c) 2015, Seth Morabito <web@loomcom.com>
#
# Permission is hereby granted, free of charge, to any person
# obtaining a copy of this software and associated documentation files
# (the "Software"), to deal in the Software without restriction,
# including without limitation the rights to use, copy, modify, merge,
# publish, distribute, sublicense, and/or sell copies of the Software,
# and to permit persons to whom the Software is furnished to do so,
# subject to the following conditions:
#
# The above copyright notice and this permission notice shall be
# included in all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
# EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
# MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
# NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
# BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
# ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
# CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.
#
##############################################################################
#
# A very bare-bones WE32100 Assembler meant for SIMH testing.
#
# The output of this assembler is not bytecode, but rather loadable
# SIMH init file format. For example, the instruction:
#
#     MOVH $3e00,4(%r0)
#
# Becomes the byte stream:
#
#     86 7F 00 3E 00 00 C1 04
#
# Becomes the output:
#
#     d -b 00 86
#     d -b 01 7F
#     d -b 02 00
#     d -b 03 3E
#     d -b 04 00
#     d -b 05 00
#     d -b 06 C1
#     d -b 07 04
#
# Here's an example of an expected input file.
#
#   {
#       "MOVW Immediate #1": {
#           "setup": [
#               ["d r5 100"]
#           ],
#           "body": [
#               "MOVW &0x100,%r0"
#           ],
#           "asserts": [
#               ["r0", "100"]
#           ]
#       },
#       "MOVW Immediate #2": {
#           "setup": [
#               ["d r5 100"]
#           ],
#           "body": [
#               "MOVW &0x1ff,%r0",
#               "MOVW &0x100,%r1"
#           ],
#           "asserts": [
#               ["r0", "1ff"],
#               ["r1", "100"]
#           ]
#       }
#   }
#
#
# To use:
#
#     we32as.rb [-s SKIP_BYTES] <input_file>
#
##############################################################################

require 'json'
require 'optparse'

#
# Monkey patch Numeric with some convenient 2's complement
# features.
#
class Encoder
  @@registers = {
    "%r0" => 0,
    "%r1" => 1,
    "%r2" => 2,
    "%r3" => 3,
    "%r4" => 4,
    "%r5" => 5,
    "%r6" => 6,
    "%r7" => 7,
    "%r8" => 8,
    "%r9" => 9,
    "%r10" => 10,
    "%r11" => 11,
    "%r12" => 12,
    "%r13" => 13,
    "%r14" => 14,
    "%r15" => 15,
    "%fp" => 9,
    "%ap" => 10,
    "%psw" => 11,
    "%sp" => 12,
    "%pcsb" => 13,
    "%isp" => 14,
    "%pc" => 15
  }

  @@opcodes = {
    MVERNO: 0x3009, ENBVJMP: 0x300D, DISVJMP: 0x3013, MOVBLW: 0x3019,
    STREND: 0x301F, INTACK: 0x302F, RETPS: 0x30C8, STRCPY: 0x3035,
    RETG: 0x3045, GATE: 0x3061, CALLPS: 0x30AC,
    SPOPRD: 0x02, SPOPD2: 0x03, MOVAW: 0x04, SPOPRT: 0x06,
    SPOPT2: 0x07, RET: 0x08, MOVTRW: 0x0c, SAVE: 0x10,
    SPOPWD: 0x13, EXTOP: 0x14, SPOPWT: 0x17, RESTORE: 0x18,
    SWAPWI: 0x1c, SWAPHI: 0x1e, SWAPBI: 0x1f, POPW: 0x20,
    SPOPRS: 0x22, SPOPS2: 0x23, JMP: 0x24, TSTW: 0x28,
    TSTH: 0x2a, TSTB: 0x2b, CALL: 0x2c, BPT: 0x2e,
    WAIT: 0x2f, SPOP: 0x32, SOPOWS: 0x33, JSB: 0x34,
    BSBH: 0x36, BSBB: 0x37, BITW: 0x38, BITH: 0x3a,
    BITB: 0x3b, CMPW: 0x3c, CMPH: 0x3e, CMPB: 0x3f,
    RGEQ: 0x40, BGEH: 0x42, BGEB: 0x43, RGTR: 0x44,
    BGH: 0x46, BGB: 0x47, RLSS: 0x48, BLH: 0x4a,
    BLB: 0x4b, RLEQ: 0x4c, BLEH: 0x4e, BLEB: 0x4f,
    RGEQU: 0x50, BGEUH: 0x52, BGEUB: 0x53, RGTRU: 0x54,
    BGUH: 0x56, BGUB: 0x57, RLSSU: 0x58, BLUH: 0x5a,
    BLUB: 0x5b, RLEQU: 0x5c, BLEUH: 0x5e, BLEUB: 0x5f,
    RVC: 0x60, BVCH: 0x62, BVCB: 0x63, RNEQU: 0x64,
    BNEH: 0x66, BNEB: 0x67, RVS: 0x68, BVSH: 0x6a,
    BVSB: 0x6b, REQLU: 0x6c, BEH: 0x6e, BEB: 0x6f,
    NOP: 0x70, NOP3: 0x72, NOP2: 0x73, RNEQ: 0x74,
    RSB: 0x78, BRH: 0x7a, BRB: 0x7b, REQL: 0x7c,
    CLRW: 0x80, CLRH: 0x82, CLRB: 0x83, MOVW: 0x84,
    MOVH: 0x86, MOVB: 0x87, MCOMW: 0x88, MCOMH: 0x8a,
    MCOMB: 0x8b, MNEGW: 0x8c, MNEGH: 0x8e, MNEGB: 0x8f,
    INCW: 0x90, INCH: 0x92, INCB: 0x93, DECW: 0x94,
    DECH: 0x96, DECB: 0x97, ADDW2: 0x9c, ADDH2: 0x9e,
    ADDB2: 0x9f, PUSHW: 0xa0, MODW2: 0xa4, MODH2: 0xa6,
    MODB2: 0xa7, MULW2: 0xa8, MULH2: 0xaa, MULB2: 0xab,
    DIVW2: 0xac, DIVH2: 0xae, DIVB2: 0xaf, ORW2: 0xb0,
    ORH2: 0xb2, ORB2: 0xb3, XORW2: 0xb4, XORH2: 0xb6,
    XORB2: 0xb7, ANDW2: 0xb8, ANDH2: 0xba, ANDB2: 0xbb,
    SUBW2: 0xbc, SUBH2: 0xbe, SUBB2: 0xbf, ALSW3: 0xc0,
    ARSW3: 0xc4, ARSH3: 0xc6, ARSB3: 0xc7, INSFW: 0xc8,
    INSFH: 0xca, INSFB: 0xcb, EXTFW: 0xcc, EXTFH: 0xce,
    EXTFB: 0xcf, LLSW3: 0xd0, LLSH3: 0xd2, LLSB3: 0xd3,
    LRSW3: 0xd4, ROTW: 0xd8, ADDW3: 0xdc, ADDH3: 0xde,
    ADDB3: 0xdf, PUSHAW: 0xe0, MODW3: 0xe4, MODH3: 0xe6,
    MODB3: 0xe7, MULW3: 0xe8, MULH3: 0xea, MULB3: 0xeb,
    DIVW3: 0xec, DIVH3: 0xee, DIVB3: 0xef, ORW3: 0xf0,
    ORH3: 0xf2, ORB3: 0xf3, XORW3: 0xf4, XORH3: 0xf6,
    XORB3: 0xf7, ANDW3: 0xf8, ANDH3: 0xfa, ANDB3: 0xfb,
    SUBW3: 0xfc, SUBH3: 0xfe, SUBB: 0xff
  }

  attr_accessor :pc

  def initialize
    @pc = 0x2000000
  end

  # Tokenize an instruction and return an array containing 0 to n
  # elements, where the first element is the opcode and subsequent
  # elements are the operands.
  def tokenize(instruction)
    code = instruction.strip

    if code.nil? || code.empty?
      return []
    end

    elements = []

    (opcode, operands) = code.split(/\s+/)

    elements << opcode

    if operands
      operands.split(/,/).each {|op| elements << op.strip}
    end

    return elements
  end

  #
  # Parse a number string (either hex or decimal) into an integer, or
  # nil if it could not be parsed.
  #
  def parse_number(number_string)
    number_string = number_string.strip
    is_hex = /^0x/.match(number_string)
    md = /^(0x)?(-?\w*)/.match(number_string)

    if md
      if is_hex
        md[2].to_i(16)
      else
        md[2].to_i(10)
      end
    end
  end

  def deferred?(operand_token)
    operand_token[0] == "*"
  end

  def absolute?(operand_token)
    (operand_token[0] == "$" ||
     (deferred?(operand_token) &&
      operand_token[1] == "$"))
  end

  def absolute_value(operand_token)
    md = /^\*?\$(.*)/.match(operand_token)

    if md
      parse_number(md[1])
    end
  end

  def displacement?(operand_token)
    (/-?\d/.match(operand_token[0]) ||
     (deferred?(operand_token) &&
      /-?\d/.match(operand_token[1])))
  end

  def displacement_value(operand_token)
    md = /^\*?([^\(]+)\(/.match(operand_token)

    if md
      parse_number(md[1])
    end
  end


  def immediate?(operand_token)
    operand_token[0] == "&"
  end

  def immediate_value(operand_token)
    parse_number(operand_token[1..-1])
  end

  def register?(operand_token)
    operand_token[0] == "%"
  end

  def register_deferred?(operand_token)
    (operand_token[0] == "(" ||
     operand_token[1] == "%")
  end

  def expanded?(operand_token)
    operand_token[0] == "{"
  end

  def expanded_type(operand_token)
    md = /^\{(\w+)\}/.match(operand_token)

    if md
      val = 0xe0

      case md[1]
      when "sbyte"
        val | 0x07
      when "half", "shalf"
        val | 0x06
      when "word", "sword"
        val | 0x04
      when "byte", "ubyte"
        val | 0x03
      when "uhalf"
        val | 0x02
      when "uword"
        val
      end
    end
  end

  def expanded_operand(operand_token)
    md = /^\{\w+\}([^\{\}]+)/.match(operand_token)

    if md
      md[1]
    end
  end

  #
  # Return the register number referenced by an operand, or nil if
  # none is referenced.
  #
  # e.g.:  "%r10" => 10
  #        "%isp" => 14
  #        "$1500" => nil
  #
  def register_value(operand_token)
    reg_match = /(%[\d\w]+)/.match(operand_token)

    if !reg_match
      return nil
    end

    reg_name = reg_match[1]

    if !reg_name
      return nil
    end

    return @@registers[reg_name]
  end

  #
  # Return a byte array containing the fully encoded bytes of
  # a single operand.
  #
  # e.g.: "%r0" = [0x40]
  #       "&1" = [0x01]
  #       "4(%r1)" = [C1, 04]
  #       "{sbyte}*0x121F(%ap)" = [e7 ba 1f 12]
  #
  def operand_bytes(operand_token)
    bytes = []

    if absolute?(operand_token)
      num = absolute_value(operand_token)
      b = word_to_bytes(num)
      if deferred?(operand_token)
        bytes << 0xef
      else
        bytes << 0x7f
      end

      bytes += b
    elsif displacement?(operand_token)
      reg = register_value(operand_token)
      disp = displacement_value(operand_token)
      defer = deferred?(operand_token)

      if reg == 10
        # AP short offset
        bytes << (0x70 | disp & 0xf)
      elsif reg == 9
        # FP short offset
        bytes << (0x60 | disp & 0xf)
      else
        b = num_to_bytes(disp)
        case b.length
        when 1
          if defer
            bytes << (0xd0 | (reg & 0xf))
          else
            bytes << (0xc0 | (reg & 0xf))
          end
        when 2
          if defer
            bytes << (0xb0 | (reg & 0xf))
          else
            bytes << (0xa0 | (reg & 0xf))
          end
        when 3
          if defer
            bytes << (0x90 | (reg & 0xf))
          else
            bytes << (0x80 | (reg & 0xf))
          end
        end

        bytes += b
      end
    elsif immediate?(operand_token)
      num = immediate_value(operand_token)
      b = num_to_bytes(num)
      case b.length
      when 4
        bytes << 0x4f
      when 2
        bytes << 0x5f
      when 1
        if (num > 0x3f && num < 0xf0)
          bytes << 0x6f
        end
      end
      bytes += b
    elsif register?(operand_token)
      num = register_value(operand_token) & 0x0f
      bytes << (0x40 | num)
    elsif register_deferred?(operand_token)
      num = register_value(operand_token) & 0x0f
      bytes << (0x50 | num)
    elsif expanded?(operand_token)
      bytes << expanded_type(operand_token)
      bytes += operand_bytes(expanded_operand(operand_token))
    end

    bytes
  end

  def word_to_bytes(num)
    [num & 0xff,
     (num >> 8) & 0xff,
     (num >> 16) & 0xff,
     (num >> 24) & 0xff]
  end

  def num_to_bytes(num)
    if (num < 0x100)
      [num & 0xff]
    elsif (num < 0x10000)
      [num & 0xff, (num >> 8) & 0xff]
    elsif (num < 0x100000000)
      word_to_bytes(num)
    end
  end

  #
  # The heart of the assembler.
  # Take a single line of input, and convert it into a byte array.
  #
  def to_bytes(instr)
    tokens = tokenize(instr)

    if tokens.nil? || tokens.empty?
      return []
    end

    operand = @@opcodes[tokens[0].to_sym]

    if operand.nil?
      return []
    end

    # operand may be one or two bytes

    if (operand > 0xff)
      bytes = [(operand & 0xff00) >> 8, (operand & 0xff)]
    else
      bytes = [operand]
    end

    tokens[1..-1].each do |operand|
      bytes += operand_bytes(operand)
    end

    return bytes
  end

  def escape(str)
    str.gsub(/\%/, "\\%")
  end

  def print_file_header
    puts <<EOF
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;                                                                          ;;
;;        DO NOT EDIT!!!!!! THIS TEST WAS GENERATED BY we32as.rb            ;;
;;                                                                          ;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

set cpu history=100
set quiet
set nomessage
set on
on afail goto failure

EOF
  end

  def print_file_footer
        puts <<EOF
:success
echo =======
echo SUCCESS
echo =======

quit

:failure
echo =======
echo FAILURE
echo =======
show history=1

EOF
  end

  def consume_file(file)
    test_count = 0
    
    json_hash = JSON.parse(File.open(file).read)

    print_file_header()

    json_hash.each do |test_name, test_block|

      @pc = 0x2000000

      puts ";;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;"
      puts "ECHO TEST #{test_count}: #{test_name}..."
      puts

      test_count += 1

      puts "; SETUP"
      
      puts "d pc %04x" % [@pc]
      puts
      
      if test_block["setup"]
        test_block["setup"].each do |key, val|
          puts "d #{key} #{val}"
        end
        puts
      end

      if test_block["body"]
        test_block["body"].each do |instruction|
          bytes = to_bytes(instruction)

          if !bytes
            raise "Unable to assemble instruction: #{instruction}"
          end

          puts "; #{instruction}"

          bytes.each do |b|
            puts "dep -b %04x %02x" % [@pc, b]
            @pc += 1
          end
        end
        puts
      end

      puts "; Execute the instruction(s)"
      puts "STEP %d" % test_block["body"].size
      puts

      if test_block["asserts"]
        puts "; ASSERTS"
        test_block["asserts"].each do |key, val|
          puts "ASSERT #{key}=#{val}"
        end
        puts
      end
    end

    print_file_footer()
  end
end


#
# Main
#

if __FILE__ == $0
  options = {}

  OptionParser.new do |opts|
    opts.banner = "Usage: example.rb [options]"

    opts.on("-s", "--skip SKIP_BYTES", Integer, "Skip Bytes") do |s|
      options[:skip] = s
    end

  end.parse!


  # Start reading in from the input
  encoder = Encoder.new

  infile = ARGV[0]

  if infile.nil?
    raise "File name required"
  end

  encoder.consume_file(ARGV[0])

end
