#pragma once

#include <vector>
#include <iostream>
#include <iomanip>
#include <memory>
#include <string>
#include <fstream>
#include <sys/stat.h>

#include <stdio.h>
#include <time.h>
#include <stdint.h>
#include <stdlib.h>
#include <math.h>
#include <sys/stat.h>

namespace loomcom {

//
// Superblock Format.
//
// On an "init" filesystem (partition 5), this will be Block 19
//
struct superblock
{
    uint16_t  s_isize;       // Size in blocks of inode list
    uint32_t  s_fsize;       // Size in blocks of the entire volume
    uint16_t  s_nfree;       // Number of addresses in s_free
    uint32_t  s_free[50];    // Free block list
    uint16_t  s_ninode;      // Number of i-nodes in s_inode
    uint16_t  s_inode[100];  // free i-node list
    uint8_t   s_flock;       // Lock during free list manipulation
    uint8_t   s_ilock;       // Lock during i-list manipulation
    uint8_t   s_fmod;        // Super block modified flag
    uint8_t   s_ronly;       // Mounted read-only flag
    uint32_t  s_time;        // Last super block update
    uint16_t  s_dinfo[4];    // Device information
    uint32_t  s_tfree;       // total free block
    uint16_t  s_tinode;      // total free inodes
    char      s_fname[6];    // file system name
    char      s_fpack[6];    // file system pack name
    uint32_t  s_fill[12];    // adjust to make sizeof filsys
    uint32_t  s_state;       // file system state
    uint32_t  s_magic;       // magic number to indicate new file system
    uint32_t  s_type;        // type of new file system
};

//
// On-disk structure of an inode.
//
struct dinode
{
    uint16_t di_mode;        // Mode and type of file
    uint16_t di_nlink;       // Number of links to file
    uint16_t di_uid;         // Owner's User ID
    uint16_t di_gid;         // Owner's Group ID
    uint32_t di_size;        // Size of file (in bytes)
    uint8_t  di_addr[40];    // Disk block addresses
    uint32_t di_atime;       // Time last accessed
    uint32_t di_mtime;       // Time last modified
    uint32_t di_ctime;       // Time created
};

//
// On-disk structure of a directory entry.
//
struct dentry {
    uint16_t d_inum;         // Inode number
    char     d_name[14];     // Name
};

class FileEntry {
public:
    typedef std::shared_ptr<FileEntry> Ptr;
    
    FileEntry(const std::string name, uint32_t inode_num);
    ~FileEntry();
    
    const std::vector<dentry> &dir_entries() const;

    bool is_dir;
    struct dinode inode;

    // File attributes
    std::string name;
    int file_type;
    int mode;
    uint32_t inode_num;
private:
    const std::vector<dentry> dir_entries_;
};

//
// Load data from a file
//
class FileLoader {
public:
    const static int SUPERBLOCK_OFFSET = 0x2600;

    const static unsigned int FS_MAGIC = 0xfd187e20;

    const static int DIRENTRY_SIZE = 16;
    const static int INODE_SIZE = 64;
    
    FileLoader(const std::string file_name);
    ~FileLoader();

    const void load();
    const void print_superblock() const;
    const void print_inodes() const;
private:
    const uint32_t eswap32(const uint32_t val) const;
    const uint16_t eswap16(const uint16_t val) const;
    const uint32_t disk_addr(uint8_t *buf) const;
    const void read_superblock();
    const void read_root();
    const void read_inode(struct dinode &inode, const uint32_t inode_num);
    const FileEntry::Ptr read_fileentry(std::string name, uint32_t inode_num);
    const std::string file_name_;
    uint16_t block_size_;
    uint32_t inode_offset_;
    uint32_t inodes_per_block_; // How many inodes per block of the
                                // inode list

    // The superblock
    struct superblock superblock_;
    // Number of inode entries (superblock only gives us this in blocks)
    uint32_t num_inodes_;
    struct tm last_update_;

    // The root directory
    FileEntry root_;
};

}; // namespace
