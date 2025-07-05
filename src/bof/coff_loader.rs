// src/bof/coff_loader.rs - COFF Object File Loader for BOF Execution (FIXED)
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void, c_int};
use std::ptr;
use std::mem;

// COFF File Header Structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct CoffFileHeader {
    pub machine: u16,
    pub number_of_sections: u16,
    pub time_date_stamp: u32,
    pub pointer_to_symbol_table: u32,
    pub number_of_symbols: u32,
    pub size_of_optional_header: u16,
    pub characteristics: u16,
}

// COFF Section Header
#[repr(C)]
#[derive(Debug, Clone)]
pub struct CoffSectionHeader {
    pub name: [u8; 8],
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub size_of_raw_data: u32,
    pub pointer_to_raw_data: u32,
    pub pointer_to_relocations: u32,
    pub pointer_to_line_numbers: u32,
    pub number_of_relocations: u16,
    pub number_of_line_numbers: u16,
    pub characteristics: u32,
}

// COFF Symbol Table Entry
#[repr(C)]
#[derive(Debug, Clone)]
pub struct CoffSymbol {
    pub name: [u8; 8],
    pub value: u32,
    pub section_number: i16,
    pub symbol_type: u16,
    pub storage_class: u8,
    pub number_of_aux_symbols: u8,
}

// COFF Relocation Entry
#[repr(C)]
#[derive(Debug, Clone)]
pub struct CoffRelocation {
    pub virtual_address: u32,
    pub symbol_table_index: u32,
    pub relocation_type: u16,
}

// Beacon API function signatures (FIXED - removed variadic function)
pub type BeaconPrintfFn = unsafe extern "C" fn(c_int, *const c_char);
pub type BeaconOutputFn = unsafe extern "C" fn(c_int, *const c_char, c_int);
pub type BeaconDataParseFn = unsafe extern "C" fn(*mut DataParser, *const c_char, c_int);
pub type BeaconDataIntFn = unsafe extern "C" fn(*mut DataParser) -> c_int;
pub type BeaconDataShortFn = unsafe extern "C" fn(*mut DataParser) -> i16;
pub type BeaconDataExtractFn = unsafe extern "C" fn(*mut DataParser, *mut c_int) -> *mut c_char;

// Data parser structure for BOF arguments
#[repr(C)]
pub struct DataParser {
    pub original: *mut c_char,
    pub buffer: *mut c_char,
    pub length: c_int,
    pub size: c_int,
}

// BOF execution context
pub struct BofExecutionContext {
    pub output_buffer: Vec<u8>,
    pub error_buffer: Vec<u8>,
    pub data_parser: DataParser,
}

// COFF Loader implementation
pub struct CoffLoader {
    loaded_sections: HashMap<String, *mut c_void>,
    symbol_table: HashMap<String, *mut c_void>,
    beacon_api: BeaconApi,
}

// Beacon API implementation
pub struct BeaconApi {
    pub printf_fn: BeaconPrintfFn,
    pub output_fn: BeaconOutputFn,
    pub data_parse_fn: BeaconDataParseFn,
    pub data_int_fn: BeaconDataIntFn,
    pub data_short_fn: BeaconDataShortFn,
    pub data_extract_fn: BeaconDataExtractFn,
}

impl CoffLoader {
    pub fn new() -> Self {
        CoffLoader {
            loaded_sections: HashMap::new(),
            symbol_table: HashMap::new(),
            beacon_api: BeaconApi::new(),
        }
    }

    // Load and execute COFF BOF
    pub fn load_and_execute(&mut self, coff_data: &[u8], args: &[u8]) -> Result<Vec<u8>, String> {
        println!("🔧 Loading COFF BOF ({} bytes)", coff_data.len());
        
        // Parse COFF header
        let coff_header = self.parse_coff_header(coff_data)?;
        println!("📋 COFF Header: {:?}", coff_header);

        // Validate architecture
        self.validate_architecture(coff_header.machine)?;

        // Parse sections
        let sections = self.parse_sections(coff_data, &coff_header)?;
        println!("📦 Found {} sections", sections.len());

        // Parse symbols
        let symbols = self.parse_symbols(coff_data, &coff_header)?;
        println!("🔗 Found {} symbols", symbols.len());

        // Allocate and load sections
        self.allocate_sections(&sections, coff_data)?;

        // Resolve relocations
        self.process_relocations(coff_data, &coff_header, &sections)?;

        // Find and execute 'go' function
        self.execute_go_function(args)
    }

    // Parse COFF file header
    fn parse_coff_header(&self, data: &[u8]) -> Result<CoffFileHeader, String> {
        if data.len() < mem::size_of::<CoffFileHeader>() {
            return Err("COFF file too small".to_string());
        }

        let header = unsafe {
            ptr::read(data.as_ptr() as *const CoffFileHeader)
        };

        // Validate COFF magic numbers
        match header.machine {
            0x014c => println!("🏗️ Architecture: x86"),
            0x8664 => println!("🏗️ Architecture: x64"),
            _ => return Err(format!("Unsupported architecture: 0x{:x}", header.machine)),
        }

        Ok(header)
    }

    // Validate target architecture
    fn validate_architecture(&self, machine: u16) -> Result<(), String> {
        #[cfg(target_arch = "x86_64")]
        if machine != 0x8664 {
            return Err("x64 BOF required for x64 beacon".to_string());
        }

        #[cfg(target_arch = "x86")]
        if machine != 0x014c {
            return Err("x86 BOF required for x86 beacon".to_string());
        }

        Ok(())
    }

    // Parse COFF sections
    fn parse_sections(&self, data: &[u8], header: &CoffFileHeader) -> Result<Vec<CoffSectionHeader>, String> {
        let mut sections = Vec::new();
        let sections_offset = mem::size_of::<CoffFileHeader>() + header.size_of_optional_header as usize;

        for i in 0..header.number_of_sections {
            let section_offset = sections_offset + (i as usize * mem::size_of::<CoffSectionHeader>());
            
            if section_offset + mem::size_of::<CoffSectionHeader>() > data.len() {
                return Err("Invalid section header offset".to_string());
            }

            let section = unsafe {
                ptr::read(data.as_ptr().add(section_offset) as *const CoffSectionHeader)
            };

            let section_name = String::from_utf8_lossy(&section.name)
                .trim_end_matches('\0')
                .to_string();
            
            println!("📄 Section: {} (size: {}, offset: 0x{:x})", 
                section_name, section.size_of_raw_data, section.pointer_to_raw_data);

            sections.push(section);
        }

        Ok(sections)
    }

    // Parse COFF symbol table
    fn parse_symbols(&self, data: &[u8], header: &CoffFileHeader) -> Result<Vec<CoffSymbol>, String> {
        let mut symbols = Vec::new();
        
        if header.pointer_to_symbol_table == 0 || header.number_of_symbols == 0 {
            return Ok(symbols);
        }

        let symbol_table_offset = header.pointer_to_symbol_table as usize;
        
        for i in 0..header.number_of_symbols {
            let symbol_offset = symbol_table_offset + (i as usize * mem::size_of::<CoffSymbol>());
            
            if symbol_offset + mem::size_of::<CoffSymbol>() > data.len() {
                break;
            }

            let symbol = unsafe {
                ptr::read(data.as_ptr().add(symbol_offset) as *const CoffSymbol)
            };

            symbols.push(symbol);
        }

        println!("🔗 Parsed {} symbols", symbols.len());
        Ok(symbols)
    }

    // Allocate memory for sections
    fn allocate_sections(&mut self, sections: &[CoffSectionHeader], data: &[u8]) -> Result<(), String> {
        for section in sections {
            if section.size_of_raw_data == 0 {
                continue;
            }

            let section_name = String::from_utf8_lossy(&section.name)
                .trim_end_matches('\0')
                .to_string();

            // Allocate executable memory for .text sections, readable for others
            let protection = if section_name == ".text" {
                // PAGE_EXECUTE_READWRITE
                0x40
            } else {
                // PAGE_READWRITE  
                0x04
            };

            let section_memory = unsafe {
                self.allocate_memory(section.size_of_raw_data as usize, protection)?
            };

            // Copy section data
            if section.pointer_to_raw_data != 0 {
                let data_start = section.pointer_to_raw_data as usize;
                let data_end = data_start + section.size_of_raw_data as usize;
                
                if data_end <= data.len() {
                    unsafe {
                        ptr::copy_nonoverlapping(
                            data.as_ptr().add(data_start),
                            section_memory as *mut u8,
                            section.size_of_raw_data as usize,
                        );
                    }
                }
            }

            self.loaded_sections.insert(section_name.clone(), section_memory);
            println!("💾 Allocated section '{}' at 0x{:p}", section_name, section_memory);
        }

        Ok(())
    }

    // Allocate executable memory
    unsafe fn allocate_memory(&self, size: usize, protection: u32) -> Result<*mut c_void, String> {
        #[cfg(windows)]
        {
            use std::os::windows::raw::HANDLE;
            type LPVOID = *mut c_void;
            type SIZE_T = usize;
            type DWORD = u32;
            
            extern "system" {
                fn VirtualAlloc(
                    lpAddress: LPVOID,
                    dwSize: SIZE_T,
                    flAllocationType: DWORD,
                    flProtect: DWORD,
                ) -> LPVOID;
            }

            let memory = VirtualAlloc(
                ptr::null_mut(),
                size,
                0x1000 | 0x2000, // MEM_COMMIT | MEM_RESERVE
                protection,
            );

            if memory.is_null() {
                return Err("Failed to allocate memory".to_string());
            }

            Ok(memory)
        }

        #[cfg(unix)]
        {
            use std::os::raw::c_int;
            
            extern "C" {
                fn mmap(
                    addr: *mut c_void,
                    length: usize,
                    prot: c_int,
                    flags: c_int,
                    fd: c_int,
                    offset: isize,
                ) -> *mut c_void;
            }

            let prot = if protection == 0x40 {
                0x1 | 0x2 | 0x4 // PROT_READ | PROT_WRITE | PROT_EXEC
            } else {
                0x1 | 0x2 // PROT_READ | PROT_WRITE
            };

            let memory = mmap(
                ptr::null_mut(),
                size,
                prot,
                0x02 | 0x20, // MAP_PRIVATE | MAP_ANONYMOUS
                -1,
                0,
            );

            if memory == ptr::null_mut() || memory == (-1isize) as *mut c_void {
                return Err("Failed to allocate memory".to_string());
            }

            Ok(memory)
        }
    }

    // Process relocations
    fn process_relocations(&self, data: &[u8], header: &CoffFileHeader, sections: &[CoffSectionHeader]) -> Result<(), String> {
        for section in sections {
            if section.number_of_relocations == 0 {
                continue;
            }

            let section_name = String::from_utf8_lossy(&section.name)
                .trim_end_matches('\0')
                .to_string();

            println!("🔗 Processing {} relocations for section '{}'", 
                section.number_of_relocations, section_name);

            let reloc_offset = section.pointer_to_relocations as usize;
            
            for i in 0..section.number_of_relocations {
                let relocation_offset = reloc_offset + (i as usize * mem::size_of::<CoffRelocation>());
                
                if relocation_offset + mem::size_of::<CoffRelocation>() > data.len() {
                    continue;
                }

                let relocation = unsafe {
                    ptr::read(data.as_ptr().add(relocation_offset) as *const CoffRelocation)
                };

                self.apply_relocation(&section_name, &relocation, data, header)?;
            }
        }

        Ok(())
    }

    // Apply individual relocation
    fn apply_relocation(&self, section_name: &str, relocation: &CoffRelocation, data: &[u8], header: &CoffFileHeader) -> Result<(), String> {
        let section_base = self.loaded_sections.get(section_name)
            .ok_or_else(|| format!("Section '{}' not found", section_name))?;

        // Get symbol value
        let symbol_value = self.resolve_symbol(relocation.symbol_table_index, data, header)?;

        // Calculate relocation address
        let reloc_address = unsafe {
            (*section_base as *mut u8).add(relocation.virtual_address as usize)
        };

        // Apply relocation based on type
        unsafe {
            match relocation.relocation_type {
                // IMAGE_REL_AMD64_ADDR64 or IMAGE_REL_I386_DIR32
                0x01 | 0x06 => {
                    *(reloc_address as *mut u64) = symbol_value as u64;
                }
                // IMAGE_REL_AMD64_REL32 or IMAGE_REL_I386_REL32
                0x04 | 0x14 => {
                    let current_value = *(reloc_address as *mut u32);
                    let new_value = (symbol_value as i64 - reloc_address as i64 - 4) as u32;
                    *(reloc_address as *mut u32) = current_value.wrapping_add(new_value);
                }
                _ => {
                    println!("⚠️ Unsupported relocation type: 0x{:x}", relocation.relocation_type);
                }
            }
        }

        Ok(())
    }

    // Resolve symbol value
    fn resolve_symbol(&self, symbol_index: u32, data: &[u8], header: &CoffFileHeader) -> Result<usize, String> {
        if header.pointer_to_symbol_table == 0 {
            return Err("No symbol table".to_string());
        }

        let symbol_offset = header.pointer_to_symbol_table as usize + 
            (symbol_index as usize * mem::size_of::<CoffSymbol>());

        if symbol_offset + mem::size_of::<CoffSymbol>() > data.len() {
            return Err("Invalid symbol index".to_string());
        }

        let symbol = unsafe {
            ptr::read(data.as_ptr().add(symbol_offset) as *const CoffSymbol)
        };

        // Get symbol name
        let symbol_name = if symbol.name[0] == 0 && symbol.name[1] == 0 &&
                             symbol.name[2] == 0 && symbol.name[3] == 0 {
            // Long name stored in string table
            let string_table_offset = u32::from_le_bytes([
                symbol.name[4], symbol.name[5], symbol.name[6], symbol.name[7]
            ]) as usize;
            
            let string_table_base = header.pointer_to_symbol_table as usize + 
                (header.number_of_symbols as usize * mem::size_of::<CoffSymbol>());
            
            let name_offset = string_table_base + string_table_offset;
            
            if name_offset < data.len() {
                let name_bytes = &data[name_offset..];
                let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_bytes.len());
                String::from_utf8_lossy(&name_bytes[..name_end]).to_string()
            } else {
                format!("INVALID_NAME_{}", symbol_index)
            }
        } else {
            // Short name stored directly
            let name_end = symbol.name.iter().position(|&b| b == 0).unwrap_or(8);
            String::from_utf8_lossy(&symbol.name[..name_end]).to_string()
        };

        // Resolve symbol based on section and type
        if symbol.section_number > 0 {
            // Symbol in a section - resolve to loaded section base + value
            let _section_index = (symbol.section_number - 1) as usize;
            // This is simplified - in a real implementation, you'd need to track section mappings
            Ok(symbol.value as usize)
        } else if symbol.section_number == 0 {
            // External symbol - resolve from beacon API or system libraries
            self.resolve_external_symbol(&symbol_name)
        } else {
            // Absolute symbol
            Ok(symbol.value as usize)
        }
    }

    // Resolve external symbols (Beacon API functions)
    fn resolve_external_symbol(&self, name: &str) -> Result<usize, String> {
        match name {
            "BeaconPrintf" => Ok(self.beacon_api.printf_fn as usize),
            "BeaconOutput" => Ok(self.beacon_api.output_fn as usize),
            "BeaconDataParse" => Ok(self.beacon_api.data_parse_fn as usize),
            "BeaconDataInt" => Ok(self.beacon_api.data_int_fn as usize),
            "BeaconDataShort" => Ok(self.beacon_api.data_short_fn as usize),
            "BeaconDataExtract" => Ok(self.beacon_api.data_extract_fn as usize),
            // Add more Beacon API functions as needed
            "LoadLibraryA" | "GetProcAddress" | "VirtualAlloc" | "VirtualFree" => {
                // Resolve system functions
                self.resolve_system_function(name)
            }
            _ => {
                println!("⚠️ Unresolved symbol: {}", name);
                Ok(0) // Return null for unresolved symbols
            }
        }
    }

    // Resolve system API functions
    fn resolve_system_function(&self, name: &str) -> Result<usize, String> {
        #[cfg(windows)]
        {
            use std::ffi::CString;
            
            extern "system" {
                fn GetModuleHandleA(lpModuleName: *const c_char) -> *mut c_void;
                fn GetProcAddress(hModule: *mut c_void, lpProcName: *const c_char) -> *mut c_void;
            }

            let (module_name, func_name) = match name {
                "LoadLibraryA" | "GetProcAddress" | "VirtualAlloc" | "VirtualFree" => ("kernel32.dll", name),
                _ => return Err(format!("Unknown system function: {}", name)),
            };

            let module_cstr = CString::new(module_name).unwrap();
            let func_cstr = CString::new(func_name).unwrap();

            unsafe {
                let module = GetModuleHandleA(module_cstr.as_ptr());
                if module.is_null() {
                    return Err(format!("Module {} not found", module_name));
                }

                let func_addr = GetProcAddress(module, func_cstr.as_ptr());
                if func_addr.is_null() {
                    return Err(format!("Function {} not found", func_name));
                }

                Ok(func_addr as usize)
            }
        }

        #[cfg(unix)]
        {
            // For Unix systems, you'd use dlsym to resolve symbols
            println!("⚠️ System function resolution not implemented for Unix: {}", name);
            Ok(0)
        }
    }

    // Find and execute the 'go' function
    fn execute_go_function(&self, args: &[u8]) -> Result<Vec<u8>, String> {
        let text_section = self.loaded_sections.get(".text")
            .ok_or_else(|| "No .text section found".to_string())?;

        // In a real implementation, you'd find the 'go' function symbol
        // For now, assume it's at the beginning of .text section
        let go_function: unsafe extern "C" fn(*mut c_char, c_int) = unsafe {
            mem::transmute(*text_section)
        };

        println!("🚀 Executing BOF 'go' function...");

        // Create execution context
        let mut context = BofExecutionContext {
            output_buffer: Vec::new(),
            error_buffer: Vec::new(),
            data_parser: DataParser {
                original: ptr::null_mut(),
                buffer: ptr::null_mut(),
                length: 0,
                size: 0,
            },
        };

        // Set up global context for beacon API callbacks
        unsafe {
            EXECUTION_CONTEXT = &mut context as *mut BofExecutionContext;
        }

        // Execute the BOF
        unsafe {
            let args_ptr = args.as_ptr() as *mut c_char;
            go_function(args_ptr, args.len() as c_int);
        }

        // Reset context
        unsafe {
            EXECUTION_CONTEXT = ptr::null_mut();
        }

        println!("✅ BOF execution completed");
        Ok(context.output_buffer)
    }
}

// Global execution context for beacon API callbacks
static mut EXECUTION_CONTEXT: *mut BofExecutionContext = ptr::null_mut();

impl BeaconApi {
    pub fn new() -> Self {
        BeaconApi {
            printf_fn: beacon_printf_impl,
            output_fn: beacon_output_impl,
            data_parse_fn: beacon_data_parse_impl,
            data_int_fn: beacon_data_int_impl,
            data_short_fn: beacon_data_short_impl,
            data_extract_fn: beacon_data_extract_impl,
        }
    }
}

// Beacon API implementation functions (FIXED - removed variadic args)
unsafe extern "C" fn beacon_printf_impl(msg_type: c_int, format: *const c_char) {
    if EXECUTION_CONTEXT.is_null() {
        return;
    }

    let context = &mut *EXECUTION_CONTEXT;
    
    // Simple implementation - no variadic args
    if !format.is_null() {
        let format_str = CStr::from_ptr(format).to_string_lossy();
        let output = format!("[{}] {}\n", msg_type, format_str);
        
        if msg_type == 0x0d { // CALLBACK_ERROR
            context.error_buffer.extend_from_slice(output.as_bytes());
        } else {
            context.output_buffer.extend_from_slice(output.as_bytes());
        }
    }
}

unsafe extern "C" fn beacon_output_impl(msg_type: c_int, data: *const c_char, length: c_int) {
    if EXECUTION_CONTEXT.is_null() || data.is_null() || length <= 0 {
        return;
    }

    let context = &mut *EXECUTION_CONTEXT;
    let data_slice = std::slice::from_raw_parts(data as *const u8, length as usize);
    
    if msg_type == 0x0d { // CALLBACK_ERROR
        context.error_buffer.extend_from_slice(data_slice);
    } else {
        context.output_buffer.extend_from_slice(data_slice);
    }
}

unsafe extern "C" fn beacon_data_parse_impl(parser: *mut DataParser, buffer: *const c_char, size: c_int) {
    if parser.is_null() || buffer.is_null() {
        return;
    }

    (*parser).original = buffer as *mut c_char;
    (*parser).buffer = buffer as *mut c_char;
    (*parser).length = size;
    (*parser).size = size;
}

unsafe extern "C" fn beacon_data_int_impl(parser: *mut DataParser) -> c_int {
    if parser.is_null() || (*parser).buffer.is_null() || (*parser).length < 4 {
        return 0;
    }

    let value = *((*parser).buffer as *const i32);
    (*parser).buffer = (*parser).buffer.add(4);
    (*parser).length -= 4;
    
    value
}

unsafe extern "C" fn beacon_data_short_impl(parser: *mut DataParser) -> i16 {
    if parser.is_null() || (*parser).buffer.is_null() || (*parser).length < 2 {
        return 0;
    }

    let value = *((*parser).buffer as *const i16);
    (*parser).buffer = (*parser).buffer.add(2);
    (*parser).length -= 2;
    
    value
}

unsafe extern "C" fn beacon_data_extract_impl(parser: *mut DataParser, size: *mut c_int) -> *mut c_char {
    if parser.is_null() || (*parser).buffer.is_null() || (*parser).length < 4 {
        if !size.is_null() {
            *size = 0;
        }
        return ptr::null_mut();
    }

    // Read size
    let data_size = *((*parser).buffer as *const i32);
    (*parser).buffer = (*parser).buffer.add(4);
    (*parser).length -= 4;

    if (*parser).length < data_size {
        if !size.is_null() {
            *size = 0;
        }
        return ptr::null_mut();
    }

    let data_ptr = (*parser).buffer;
    (*parser).buffer = (*parser).buffer.add(data_size as usize);
    (*parser).length -= data_size;

    if !size.is_null() {
        *size = data_size;
    }

    data_ptr
}

// Helper function to create a complete BOF execution environment
pub fn create_bof_runtime() -> CoffLoader {
    let mut loader = CoffLoader::new();
    
    // Pre-populate common symbols that BOFs might need
    loader.symbol_table.insert("BeaconPrintf".to_string(), beacon_printf_impl as *mut c_void);
    loader.symbol_table.insert("BeaconOutput".to_string(), beacon_output_impl as *mut c_void);
    loader.symbol_table.insert("BeaconDataParse".to_string(), beacon_data_parse_impl as *mut c_void);
    loader.symbol_table.insert("BeaconDataInt".to_string(), beacon_data_int_impl as *mut c_void);
    loader.symbol_table.insert("BeaconDataShort".to_string(), beacon_data_short_impl as *mut c_void);
    loader.symbol_table.insert("BeaconDataExtract".to_string(), beacon_data_extract_impl as *mut c_void);
    
    println!("🏗️ BOF Runtime initialized with Beacon API");
    loader
}

// Example usage for common BOF patterns
impl CoffLoader {
    pub fn execute_recon_bof(&mut self, bof_data: &[u8], target: &str) -> Result<Vec<u8>, String> {
        let mut args = vec![];
        
        // Pack target parameter
        let target_cstr = CString::new(target).unwrap();
        let target_bytes = target_cstr.as_bytes_with_nul();
        args.extend_from_slice(&(target_bytes.len() as u32).to_le_bytes());
        args.extend_from_slice(target_bytes);
        
        self.load_and_execute(bof_data, &args)
    }

    pub fn execute_command_bof(&mut self, bof_data: &[u8], command: &str) -> Result<Vec<u8>, String> {
        let mut args = vec![];
        
        // Pack command parameter
        let cmd_cstr = CString::new(command).unwrap();
        let cmd_bytes = cmd_cstr.as_bytes_with_nul();
        args.extend_from_slice(&(cmd_bytes.len() as u32).to_le_bytes());
        args.extend_from_slice(cmd_bytes);
        
        self.load_and_execute(bof_data, &args)
    }
}