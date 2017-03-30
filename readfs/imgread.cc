#include "imgread.hh"

namespace loomcom {


//////////////////////////////////////////////////////////////////////
// FileEntry
//

FileEntry::FileEntry(const std::string name, uint32_t inode_num) :
    name(name), inode_num(inode_num)
{
}

FileEntry::~FileEntry()
{
}

const std::vector<dentry> &FileEntry::dir_entries() const
{
    return dir_entries_;
}

//////////////////////////////////////////////////////////////////////
// FileLoader
//

FileLoader::FileLoader(const std::string file_name) :
    file_name_(file_name),
    root_("/", 1)
{
}

FileLoader::~FileLoader()
{
}

const void FileLoader::load()
{
    std::cout << "Loading file " << file_name_ << std::endl;

    // The first thing we do is read the superblock.
    read_superblock();
    print_superblock();

    // Now read the root file entry
    read_root();
}

const void FileLoader::read_superblock()
{
    std::fstream file;
    file.open(file_name_.c_str(), std::ifstream::in | std::ifstream::binary);
    file.seekg(SUPERBLOCK_OFFSET, file.beg);

    if ((file.rdstate() & std::ifstream::eofbit) != 0) {
        std::cerr << "Failed to read superblock." << std::endl;
        throw std::exception();
    }

    file.read(reinterpret_cast<char *>(&superblock_), sizeof(struct superblock));

    file.close();

    superblock_.s_isize  = eswap16(superblock_.s_isize);
    superblock_.s_fsize  = eswap32(superblock_.s_fsize);
    superblock_.s_nfree  = eswap16(superblock_.s_nfree);
    superblock_.s_ninode = eswap16(superblock_.s_ninode);
    superblock_.s_time   = eswap32(superblock_.s_time);
    superblock_.s_tfree  = eswap32(superblock_.s_tfree);
    superblock_.s_tinode = eswap16(superblock_.s_tinode);
    superblock_.s_state  = eswap32(superblock_.s_state);
    superblock_.s_magic  = eswap32(superblock_.s_magic);
    superblock_.s_type   = eswap32(superblock_.s_type);

    switch (superblock_.s_type) {
    case 1:
        block_size_ = 512;
        inode_offset_ = 512 * 20;
        break;
    case 2:
        block_size_ = 1024;
        inode_offset_ = 512 * 22;
        break;
    default:
        block_size_ = 1024;
        inode_offset_ = 512 * 22;
    }
    
    // Check for MAGIC
    if (superblock_.s_magic != FS_MAGIC) {
        std::cerr << "Does not appear to be a SysV filesystem!" << std::endl;
        throw std::exception();
    }

    for (int i = 0; i < 50; i++) {
        superblock_.s_free[i] = eswap32(superblock_.s_free[i]);
    }

    for (int i = 0; i < 100; i++) {
        superblock_.s_inode[i] = eswap16(superblock_.s_inode[i]);
    }

    for (int i = 0; i < 4; i++) {
        superblock_.s_dinfo[i] = eswap16(superblock_.s_dinfo[i]);
    }

    // Calculate the number of inode entries
    num_inodes_ = (superblock_.s_isize * block_size_) / DIRENTRY_SIZE;
    inodes_per_block_ = block_size_ / INODE_SIZE;

    time_t t = (time_t)superblock_.s_time;
    last_update_ = *localtime(&t);
}

const void FileLoader::read_inode(struct dinode &inode, const uint32_t inode_num)
{
    std::fstream file;
    int offset = inode_offset_ + (inode_num * INODE_SIZE);

    file.open(file_name_.c_str(), std::ifstream::in | std::ifstream::binary);
    file.seekg(offset, file.beg);
    
    if ((file.rdstate() & std::ifstream::eofbit) != 0) {
        std::cerr << "Failed to read inode " << inode_num << std::endl;
        throw std::exception();
    }

    file.read(reinterpret_cast<char *>(&inode), sizeof(struct dinode));

    file.close();

    // Correct endianness
    inode.di_mode  = eswap16(inode.di_mode);
    inode.di_nlink = eswap16(inode.di_nlink);
    inode.di_uid   = eswap16(inode.di_uid);
    inode.di_gid   = eswap16(inode.di_gid);
    inode.di_size  = eswap32(inode.di_size);
    inode.di_atime = eswap32(inode.di_atime);
    inode.di_mtime = eswap32(inode.di_mtime);
    inode.di_ctime = eswap32(inode.di_ctime);
}

const void FileLoader::read_root()
{
    std::fstream file;

    read_inode(root_.inode, 1);
    
    // Set some file attributes on the root.
    root_.file_type = (0xf000 & root_.inode.di_mode) >> 12;
    root_.mode = 0xfff & root_.inode.di_mode;

    // Now load the root's directory entries.
    int block_count = (int) ceil(root_.inode.di_size / (float) block_size_);
    int entry_count =  root_.inode.di_size / DIRENTRY_SIZE;
    int entries_per_block =  block_size_ / DIRENTRY_SIZE;

    std::cout << " [DBG] Root contains " << std::dec << entry_count << " entries" << std::endl;
    std::cout << " [DBG] Root is " << block_count << " block(s) long" << std::endl;
    std::cout << " [DBG] Each block contains at most " << entries_per_block << " entries." << std::endl;

    // TODO: Support for root directories with more than 10 blocks.
    if (block_count > 10) {
        throw std::exception();
    }
    
    for (int block_num = 0; block_num < block_count; block_num++) {
        int addr = disk_addr(root_.inode.di_addr + (block_num * 3));
        int offset = 0x2400 + (addr * block_size_);
        int entries_this_block = 0;

        // How many entries are in _this_ block?
        // We know the total number of entries in the root inode is
        // "entry_count", and we know that each block can contain up
        // to "entries_per_block" entries.
        if (block_num == block_count - 1) {
            entries_this_block = entry_count % entries_per_block;
        } else {
            entries_this_block = entries_per_block;
        }

        std::cout << " [DBG] Root block #" << block_num << " address is " <<
            addr << std::endl;
        std::cout << " [DBG] Root block #" << block_num << " offset is 0x" <<
            std::hex << offset << std::endl;

        // Read in the file names!
        for (int i =  0; i < entries_this_block; i++) {
            // Read in the direntry.
            struct dentry entry;

            file.open(file_name_.c_str(), std::ifstream::in | std::ifstream::binary);
            file.seekg(offset + (i * sizeof(struct dentry)), file.beg);
    
            if ((file.rdstate() & std::ifstream::eofbit) != 0) {
                std::cerr << "Failed to read directory entry." << std::endl;
                throw std::exception();
            }
            
            file.read(reinterpret_cast<char *>(&entry), sizeof(struct dentry));
            file.close();

            // Create the FileEntry object
            FileEntry::Ptr f = read_fileentry(entry.d_name, eswap16(entry.d_inum));

            std::cout << std::setfill(' ');
            std::cout << " [DBG]  ";
            std::cout << std::setw(3) << std::dec << f->inode_num << " ";
            std::cout << std::setw(14) << f->name << " ";
            std::cout << std::setw(2) << f->file_type << " ";
            std::cout << std::setw(4) << std::setfill('0') << std::oct << f->mode;


            std::cout << std::endl;
        }
    }
}


const FileEntry::Ptr FileLoader::read_fileentry(std::string name, uint32_t inode_num)
{
    FileEntry::Ptr file_entry = std::make_shared<FileEntry>(name, inode_num);

    read_inode(file_entry->inode, inode_num);

    file_entry->file_type = (0xf000 & file_entry->inode.di_mode) >> 12;
    file_entry->mode = 0x0fff & file_entry->inode.di_mode;
    file_entry->is_dir = file_entry->file_type == 8;
    
    return file_entry;
}

const void FileLoader::print_superblock() const
{
    char time_str[100];
    strftime(time_str, sizeof(time_str), "%Y-%m-%d %H:%M:%S", &last_update_);

    std::cout << "FILESYSTEM INFO" << std::endl;
    std::cout << "---------------" << std::endl;
    std::cout << "  Size in blocks of i-list: " << std::dec <<
        superblock_.s_isize << std::endl;
    std::cout << "  Size of inode list in entries: " << num_inodes_ << std::endl;
    std::cout << "  Size in blocks of entire volume: " << superblock_.s_fsize << std::endl;
    std::cout << "  Free inodes: " << superblock_.s_ninode << std::endl;
    std::cout << "  Free blocks: " << superblock_.s_nfree << std::endl;
    std::cout << "  File System Type: " << superblock_.s_type << std::endl;
    std::cout << "  File System State: " << std::hex << superblock_.s_state << std::endl;
    std::cout << "  File System Name: " << superblock_.s_fname << std::endl;
    std::cout << "  Last Superblock Update Time: " << time_str << std::endl;
}

const uint32_t FileLoader::eswap32(const uint32_t val) const
{
    return (((val & 0x000000ff) << 24) |
            ((val & 0x0000ff00) << 8) |
            ((val & 0x00ff0000) >> 8) |
            ((val & 0xff000000) >> 24));
}

const uint16_t FileLoader::eswap16(const uint16_t val) const
{
    return (((val & 0x00ff) << 8) |
            ((val & 0xff00) >> 8));
}

const uint32_t FileLoader::disk_addr(uint8_t *buf) const
{
    return buf[0] << 12 | buf[1] << 8 | buf[2];
}



}; // namespace

using namespace std;
using namespace loomcom;

void usage() {
    cerr << "Usage: imgread <file>" << endl;
}

int main(int argc, char ** argv) {
    
    // First argument is the file name.
    if (argc < 2) {
        usage();
        return 1;
    }

    char *name = argv[1];

    // If the first arg isn't a file, die.
    struct stat s;
    stat(name, &s);

    if (!S_ISREG(s.st_mode)) {
        usage();
        return 1;
    }

    FileLoader file_loader(name);
    file_loader.load();

    return 0;
}
