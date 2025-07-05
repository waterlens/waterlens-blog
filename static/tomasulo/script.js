const REGISTER_COUNT = 16;
const MEMORY_SIZE = 512;
const STATION_TYPE_ARITH = 'arith';
const STATION_TYPE_MUL_DIV = 'mul/div';
const STATION_TYPE_LOAD = 'load';
const STATION_TYPE_STORE = 'store';

class TomasuloSimulator {
    constructor() {
        this.clock = 0;
        this.instructions = [];
        this.currentInstructionIndex = 0;
        this.isRunning = false;
        this.runInterval = null;
        
        // Reservation station configuration
        this.reservationStations = {
            'Arith1': { type: STATION_TYPE_ARITH, busy: false, op: '', vj: '', vk: '', qj: '', qk: '', a: '' },
            'Arith2': { type: STATION_TYPE_ARITH, busy: false, op: '', vj: '', vk: '', qj: '', qk: '', a: '' },
            'MulDiv1': { type: STATION_TYPE_MUL_DIV, busy: false, op: '', vj: '', vk: '', qj: '', qk: '', a: '' },
            'MulDiv2': { type: STATION_TYPE_MUL_DIV, busy: false, op: '', vj: '', vk: '', qj: '', qk: '', a: '' },
            'Load1': { type: STATION_TYPE_LOAD, busy: false, op: '', vj: '', vk: '', qj: '', qk: '', a: '' },
            'Load2': { type: STATION_TYPE_LOAD, busy: false, op: '', vj: '', vk: '', qj: '', qk: '', a: '' },
            'Store1': { type: STATION_TYPE_STORE, busy: false, op: '', vj: '', vk: '', qj: '', qk: '', a: '' },
            'Store2': { type: STATION_TYPE_STORE, busy: false, op: '', vj: '', vk: '', qj: '', qk: '', a: '' }
        };
        
        // Register file (R0-R15)
        this.registers = {};
        for (let i = 0; i < REGISTER_COUNT; i++) {
            this.registers[`R${i}`] = { qi: '', value: 0 };
        }
        
        // Memory
        this.memory = {};
        for (let i = 0; i < MEMORY_SIZE; i += 4) {
            this.memory[i] = i;
        }
        
        // Instruction execution status
        this.instructionStatus = [];
        
        // Address calculation status for load/store instructions
        this.addressCalcStatus = new Map(); // stationName -> {start: clock, completed: boolean}
        
        // Execution unit latency
        this.executionDelays = {
            'ADD': 1,
            'SUB': 1,
            'MUL': 6,
            'DIV': 12,
            'LD': [1, 1], // [address calculation, memory operation]
            'ST': [1, 0], // [address calculation, memory operation] - 0 means no additional delay (has buffer)
        };
        
        this.initStationTypeMap();
        this.initializeEventListeners();
        this.updateDisplay();
    }
    
    initializeEventListeners() {
        $('#load-instructions').click(() => this.loadInstructions());
        $('#step').click(() => this.step());
        $('#run').click(() => this.toggleRun());
        $('#reset').click(() => this.reset());
        $('#clear').click(() => this.clear());
    }
    
    log(message, type = 'info') {
        const timestamp = `[${this.clock.toString().padStart(3, '0')}] `;
        const logEntry = $(`<div class="log-entry log-${type}">${timestamp}${message}</div>`);
        $('#log').append(logEntry);
        $('#log').scrollTop($('#log')[0].scrollHeight);
    }
    
    parseInstruction(instruction) {
        const trimmed = instruction.trim().toUpperCase();
        if (!trimmed) return null;
        
        const parts = trimmed.split(/[,\s]+/).filter(part => part.length > 0);
        if (parts.length < 2) return null;
        
        const op = parts[0];
        const operands = parts.slice(1);
        
        // Helper function to validate register range
        const validateRegister = (reg) => {
            if (!reg.startsWith('R')) return false;
            const regNum = parseInt(reg.substring(1));
            if (isNaN(regNum) || regNum < 0 || regNum >= REGISTER_COUNT) {
                return false;
            }
            return true;
        };
        
        // Helper function to validate memory address
        const validateMemoryAddress = (offset, baseReg) => {
            if (!validateRegister(baseReg)) return false;
            const offsetNum = parseInt(offset);
            if (isNaN(offsetNum)) return false;
            
            // Calculate final address: offset + base register value
            const baseValue = this.registers[baseReg].value;
            const finalAddress = offsetNum + baseValue;
            
            // Check address range and alignment
            if (finalAddress < 0 || finalAddress >= MEMORY_SIZE) {
                return false;
            }
            if (finalAddress % 4 !== 0) {
                return false;
            }
            return true;
        };
        
        // Helper function to get memory address validation error message
        const getMemoryAddressError = (offset, baseReg) => {
            if (!validateRegister(baseReg)) {
                return `Base register ${baseReg} out of range (R0-R15)`;
            }
            const offsetNum = parseInt(offset);
            if (isNaN(offsetNum)) {
                return `Offset ${offset} is not a valid number`;
            }
            
            const baseValue = this.registers[baseReg].value;
            const finalAddress = offsetNum + baseValue;
            
            if (finalAddress < 0 || finalAddress >= MEMORY_SIZE) {
                return `Memory address ${finalAddress} (${offset}+${baseValue}) out of range`;
            }
            if (finalAddress % 4 !== 0) {
                return `Memory address ${finalAddress} (${offset}+${baseValue}) not 4-byte aligned`;
            }
            return null;
        };
        
        switch (op) {
            case 'ADD':
            case 'SUB':
                if (operands.length !== 3) return null;
                // Validate all registers are within R0-R15 range
                if (!validateRegister(operands[0]) || !validateRegister(operands[1]) || !validateRegister(operands[2])) {
                    return null;
                }
                return {
                    type: STATION_TYPE_ARITH,
                    op: op,
                    rd: operands[0],
                    rs: operands[1],
                    rt: operands[2],
                    original: instruction
                };
            case 'MUL':
            case 'DIV':
                if (operands.length !== 3) return null;
                // Validate all registers are within R0-R15 range
                if (!validateRegister(operands[0]) || !validateRegister(operands[1]) || !validateRegister(operands[2])) {
                    return null;
                }
                return {
                    type: STATION_TYPE_MUL_DIV,
                    op: op,
                    rd: operands[0],
                    rs: operands[1],
                    rt: operands[2],
                    original: instruction
                };
            
            case 'LD':
                if (operands.length !== 2) return null;
                const loadMatch = operands[1].match(/(\d+)\(([^)]+)\)/);
                if (!loadMatch) return null;
                // Validate destination register is within R0-R15 range
                if (!validateRegister(operands[0])) return null;
                // Validate memory address
                const loadError = getMemoryAddressError(loadMatch[1], loadMatch[2]);
                if (loadError) return null;
                return {
                    type: STATION_TYPE_LOAD,
                    op: 'LD',
                    rd: operands[0],
                    offset: parseInt(loadMatch[1]),
                    base: loadMatch[2],
                    original: instruction
                };
            
            case 'ST':
                if (operands.length !== 2) return null;
                const storeMatch = operands[1].match(/(\d+)\(([^)]+)\)/);
                if (!storeMatch) return null;
                // Validate source register is within R0-R15 range
                if (!validateRegister(operands[0])) return null;
                // Validate memory address
                const storeError = getMemoryAddressError(storeMatch[1], storeMatch[2]);
                if (storeError) return null;
                return {
                    type: STATION_TYPE_STORE,
                    op: 'ST',
                    rs: operands[0],
                    offset: parseInt(storeMatch[1]),
                    base: storeMatch[2],
                    original: instruction
                };
            
            default:
                return null;
        }
    }
    
    loadInstructions() {
        const instructionText = $('#instructions').val();
        const lines = instructionText.split('\n');
        
        this.instructions = [];
        this.instructionStatus = [];
        
        for (let i = 0; i < lines.length; i++) {
            const line = lines[i].trim();
            if (!line || line.startsWith('#')) continue; // Skip empty lines and comments
            
            const parsed = this.parseInstruction(line);
            if (parsed) {
                this.instructions.push(parsed);
                this.instructionStatus.push({
                    instruction: parsed.original,
                    issue: '',
                    exec: '',
                    writeback: '',
                    status: 'pending',
                    station: '',
                    execStart: 0,
                    execEnd: 0
                });
            } else {
                // Try to get more detailed error information
                const errorMsg = this.getDetailedParseError(line);
                this.log(`Instruction parsing failed: ${line} - ${errorMsg}`, 'error');
            }
        }
        
        this.log(`Loaded ${this.instructions.length} instructions`, 'success');
        this.updateDisplay();
    }
    
    getDetailedParseError(instruction) {
        const trimmed = instruction.trim().toUpperCase();
        if (!trimmed) return 'Empty instruction';
        
        const parts = trimmed.split(/[,\s]+/).filter(part => part.length > 0);
        if (parts.length < 2) return 'Invalid instruction format';
        
        const op = parts[0];
        const operands = parts.slice(1);
        
        // Helper function to validate register range
        const validateRegister = (reg) => {
            if (!reg.startsWith('R')) return false;
            const regNum = parseInt(reg.substring(1));
            if (isNaN(regNum) || regNum < 0 || regNum >= REGISTER_COUNT) {
                return false;
            }
            return true;
        };
        
        // Helper function to get memory address validation error message
        const getMemoryAddressError = (offset, baseReg) => {
            if (!validateRegister(baseReg)) {
                return `Base register ${baseReg} out of range (R0-R15)`;
            }
            const offsetNum = parseInt(offset);
            if (isNaN(offsetNum)) {
                return `Offset ${offset} is not a valid number`;
            }
            
            const baseValue = this.registers[baseReg].value;
            const finalAddress = offsetNum + baseValue;
            
            if (finalAddress < 0 || finalAddress >= MEMORY_SIZE) {
                return `Memory address ${finalAddress} (${offset}+${baseValue}) out of range (0-${MEMORY_SIZE-1})`;
            }
            if (finalAddress % 4 !== 0) {
                return `Memory address ${finalAddress} (${offset}+${baseValue}) not 4-byte aligned`;
            }
            return null;
        };
        
        switch (op) {
            case 'ADD':
            case 'SUB':
            case 'MUL':
            case 'DIV':
                if (operands.length !== 3) return 'Arithmetic instruction requires 3 operands';
                if (!validateRegister(operands[0])) return `Destination register ${operands[0]} out of range (R0-R15)`;
                if (!validateRegister(operands[1])) return `Source register 1 ${operands[1]} out of range (R0-R15)`;
                if (!validateRegister(operands[2])) return `Source register 2 ${operands[2]} out of range (R0-R15)`;
                return 'Unknown error';
            
            case 'LD':
                if (operands.length !== 2) return 'Load instruction requires 2 operands';
                const loadMatch = operands[1].match(/(\d+)\(([^)]+)\)/);
                if (!loadMatch) return 'Invalid memory address format, should be offset(base)';
                if (!validateRegister(operands[0])) return `Destination register ${operands[0]} out of range (R0-R15)`;
                const loadError = getMemoryAddressError(loadMatch[1], loadMatch[2]);
                if (loadError) return loadError;
                return 'Unknown error';
            
            case 'ST':
                if (operands.length !== 2) return 'Store instruction requires 2 operands';
                const storeMatch = operands[1].match(/(\d+)\(([^)]+)\)/);
                if (!storeMatch) return 'Invalid memory address format, should be offset(base)';
                if (!validateRegister(operands[0])) return `Source register ${operands[0]} out of range (R0-R15)`;
                const storeError = getMemoryAddressError(storeMatch[1], storeMatch[2]);
                if (storeError) return storeError;
                return 'Unknown error';
            
            default:
                return `Unsupported opcode: ${op}`;
        }
    }

    initStationTypeMap() {
        this.stationTypeMap = {
            [STATION_TYPE_ARITH]: [],
            [STATION_TYPE_MUL_DIV]: [],
            [STATION_TYPE_LOAD]: [],
            [STATION_TYPE_STORE]: []
        };
        for (const [stationName, station] of Object.entries(this.reservationStations)) {
            this.stationTypeMap[station.type].push(stationName);
        }
    }
    
    findAvailableStation(instructionType) {
        const stations = this.stationTypeMap[instructionType];
        for (const station of stations) {
            if (!this.reservationStations[station].busy) {
                return station;
            }
        }
        return null;
    }
    
    getOperandValue(operand) {
        if (operand.startsWith('R')) {
            // Validate register range (R0-R15)
            const regNum = parseInt(operand.substring(1));
            if (regNum < 0 || regNum >= REGISTER_COUNT) {
                this.log(`Error: Register ${operand} out of range (R0-R15)`, 'error');
                return { value: 0, ready: false };
            }
            
            const reg = this.registers[operand];
            if (reg.qi === '') {
                return { value: reg.value, ready: true };
            } else {
                return { value: reg.qi, ready: false };
            }
        } else {
            return { value: parseInt(operand), ready: true };
        }
    }
    
    issue() {
        if (this.currentInstructionIndex >= this.instructions.length) {
            return false;
        }
        
        const instruction = this.instructions[this.currentInstructionIndex];
        const station = this.findAvailableStation(instruction.type);
        
        if (!station) {
            return false; // No available reservation station
        }
        
        // Allocate reservation station
        const rs = this.reservationStations[station];
        rs.busy = true;
        rs.op = instruction.op;
        
        if (instruction.type === STATION_TYPE_ARITH || instruction.type === STATION_TYPE_MUL_DIV) {
            const vj = this.getOperandValue(instruction.rs);
            const vk = this.getOperandValue(instruction.rt);

            if (vj.ready) {
                rs.vj = vj.value;
                rs.qj = '';
            } else {
                rs.vj = '';
                rs.qj = vj.value;
            }
            
            if (vk.ready) {
                rs.vk = vk.value;
                rs.qk = '';
            } else {
                rs.vk = '';
                rs.qk = vk.value;
            }

            this.registers[instruction.rd].qi = station;
        } else if (instruction.type === STATION_TYPE_LOAD) {
            const base = this.getOperandValue(instruction.base);

            if (base.ready) {
                rs.vj = base.value;
                rs.qj = '';
            } else {
                rs.vj = '';
                rs.qj = base.value;
            }

            rs.a = instruction.offset;
            this.registers[instruction.rd].qi = station;
            
        } else if (instruction.type === STATION_TYPE_STORE) {
            const value = this.getOperandValue(instruction.rs);
            const base = this.getOperandValue(instruction.base);
            
            if (value.ready) {
                rs.vk = value.value;
                rs.qk = '';
            } else {
                rs.vk = '';
                rs.qk = value.value;
            }

            if (base.ready) {
                rs.vj = base.value;
                rs.qj = '';
            } else {
                rs.vj = '';
                rs.qj = base.value;
            }

            rs.a = instruction.offset;
        }
        
        // Update instruction status
        this.instructionStatus[this.currentInstructionIndex].issue = this.clock;
        this.instructionStatus[this.currentInstructionIndex].status = 'issue';
        this.instructionStatus[this.currentInstructionIndex].station = station;
        
        this.log(`Instruction ${this.currentInstructionIndex + 1} allocated to reservation station ${station}`, 'info');
        this.currentInstructionIndex++;
        
        return true;
    }
    
    execute() {
        for (const [stationName, station] of Object.entries(this.reservationStations)) {
            if (!station.busy) continue;
            
            if (station.qj !== '' || station.qk !== '') {
                continue;
            }
            
            const instructionIndex = this.instructionStatus.findIndex(
                status => status.station === stationName && (status.status === 'issue' || status.status === 'exec')
            );
            
            if (instructionIndex === -1) continue;
            
            const status = this.instructionStatus[instructionIndex];
            const instruction = this.instructions[instructionIndex];
            
            if (status.execStart === 0) {
                status.execStart = this.clock;
                status.status = 'exec';
                this.log(`Instruction ${instructionIndex + 1} started execution (${station.op})`, 'info');
                
                // For load/store instructions, start address calculation
                if (instruction.type === STATION_TYPE_LOAD || instruction.type === STATION_TYPE_STORE) {
                    this.addressCalcStatus.set(stationName, { start: this.clock, completed: false });
                    this.log(`Instruction ${instructionIndex + 1} started address calculation`, 'info');
                }
            }
            
            // Handle address calculation for load/store instructions
            if ((instruction.type === STATION_TYPE_LOAD || instruction.type === STATION_TYPE_STORE) && 
                this.addressCalcStatus.has(stationName)) {
                const addrCalc = this.addressCalcStatus.get(stationName);
                const delays = this.executionDelays[station.op];
                const addrCalcDelay = delays[0];
                const memOpDelay = delays[1];
                
                // Address calculation phase
                if (this.clock === addrCalc.start + addrCalcDelay && !addrCalc.completed) {
                    addrCalc.completed = true;
                    const address = station.vj + station.a; // For load: base + offset, For store: base + offset
                    station.a = address; // Store calculated address in 'a' field
                    
                    // For store instructions, execution is complete after address calculation (if memOpDelay is 0)
                    if (instruction.type === STATION_TYPE_STORE && memOpDelay === 0) {
                        status.execEnd = this.clock;
                        status.status = 'writeback';
                    }
                }
                
                // Memory operation phase (for load instructions or store instructions with buffer delay)
                if (addrCalc.completed && memOpDelay > 0 && 
                    this.clock === addrCalc.start + addrCalcDelay + memOpDelay) {
                    status.execEnd = this.clock;
                    status.status = 'writeback';
                }
            } else {
                // Handle arithmetic instructions
                const delay = this.executionDelays[station.op];
                if (this.clock === status.execStart + delay) {
                    status.execEnd = this.clock;
                    status.status = 'writeback';
                }
            }
            this.log(`Instruction ${instructionIndex + 1} execution completed`, 'info');
        }
    }
    
    writeback() {
        for (const [stationName, station] of Object.entries(this.reservationStations)) {
            if (!station.busy) continue;
            
            // Find corresponding instruction
            const instructionIndex = this.instructionStatus.findIndex(
                status => status.station === stationName && status.status === 'writeback'
            );
            
            if (instructionIndex === -1) continue;
            
            const instruction = this.instructions[instructionIndex];
            const status = this.instructionStatus[instructionIndex];
            
            // Perform calculation and get result
            let result = 0;
            if (instruction.type === STATION_TYPE_ARITH || instruction.type === STATION_TYPE_MUL_DIV) {
                switch (station.op) {
                    case 'ADD':
                        result = station.vj + station.vk;
                        break;
                    case 'SUB':
                        result = station.vj - station.vk;
                        break;
                    case 'MUL':
                        result = station.vj * station.vk;
                        break;
                    case 'DIV':
                        result = station.vj / station.vk;
                        break;
                }
            } else if (instruction.type === STATION_TYPE_LOAD) {
                const address = station.a; // Use pre-calculated address
                
                // Validate memory address
                if (address < 0 || address >= MEMORY_SIZE) {
                    this.log(`Error: Memory address ${address} out of range (0-${MEMORY_SIZE-1})`, 'error');
                    result = 0;
                } else if (address % 4 !== 0) {
                    this.log(`Error: Memory address ${address} not 4-byte aligned`, 'error');
                    result = 0;
                } else {
                    result = this.memory[address] || 0;
                }
            } else if (instruction.type === STATION_TYPE_STORE) {
                const address = station.a; // Use pre-calculated address
                
                // Validate memory address
                if (address < 0 || address >= MEMORY_SIZE) {
                    this.log(`Error: Memory address ${address} out of range (0-${MEMORY_SIZE-1})`, 'error');
                    result = 0;
                } else if (address % 4 !== 0) {
                    this.log(`Error: Memory address ${address} not 4-byte aligned`, 'error');
                    result = 0;
                } else {
                    this.memory[address] = station.vk;
                    result = station.vk;
                }
            }
            
            // Write back result to register
            if (instruction.type !== STATION_TYPE_STORE) {
                const rd = instruction.rd;
                this.registers[rd].value = result;
                this.registers[rd].qi = '';
                this.log(`Instruction ${instructionIndex + 1} writeback: ${rd} = ${result}`, 'success');
            }
            
            // Broadcast result to waiting reservation stations
            this.broadcastResult(stationName, result);
            
            // Release reservation station
            station.busy = false;
            station.op = '';
            station.vj = '';
            station.vk = '';
            station.qj = '';
            station.qk = '';
            station.a = '';
            
            status.writeback = this.clock;
            status.status = 'complete';
            
            this.log(`Instruction ${instructionIndex + 1} writeback completed`, 'success');
        }
    }
    
    broadcastResult(stationName, result) {
        for (const [name, station] of Object.entries(this.reservationStations)) {
            if (station.qj === stationName) {
                station.vj = result;
                station.qj = '';
            }
            if (station.qk === stationName) {
                station.vk = result;
                station.qk = '';
            }
        }
    }
    
    step() {
        this.clock++;
        this.log(`=== Clock Cycle ${this.clock} ===`, 'info');
        
        // Execute in order: writeback -> execute -> issue
        this.writeback();
        this.execute();
        this.issue();
        
        this.updateDisplay();
        
        // Check if all instructions are completed
        const allComplete = this.instructionStatus.every(status => status.status === 'complete');
        if (allComplete && this.currentInstructionIndex >= this.instructions.length) {
            this.log('All instructions completed!', 'success');
            this.stop();
        }
    }
    
    toggleRun() {
        if (this.isRunning) {
            this.stop();
        } else {
            this.start();
        }
    }
    
    start() {
        this.isRunning = true;
        $('#run').text('Stop').addClass('running');
        this.runInterval = setInterval(() => {
            this.step();
        }, 1000);
    }
    
    stop() {
        this.isRunning = false;
        $('#run').text('Run').removeClass('running');
        if (this.runInterval) {
            clearInterval(this.runInterval);
            this.runInterval = null;
        }
    }
    
    reset() {
        this.stop();
        this.clock = 0;
        this.currentInstructionIndex = 0;
        
        // Reset reservation stations
        for (const station of Object.values(this.reservationStations)) {
            station.busy = false;
            station.op = '';
            station.vj = '';
            station.vk = '';
            station.qj = '';
            station.qk = '';
            station.a = '';
        }
        
        // Reset registers
        for (const reg of Object.values(this.registers)) {
            reg.qi = '';
            reg.value = 0;
        }
        
        // Reset instruction status
        this.instructionStatus.forEach(status => {
            status.issue = '';
            status.exec = '';
            status.writeback = '';
            status.status = 'pending';
            status.station = '';
            status.execStart = 0;
            status.execEnd = 0;
        });
        
        // Reset address calculation status
        this.addressCalcStatus.clear();
        
        this.initStationTypeMap();
        this.log('Simulator reset', 'info');
        this.updateDisplay();
    }
    
    clear() {
        this.stop();
        this.instructions = [];
        this.instructionStatus = [];
        this.currentInstructionIndex = 0;
        this.clock = 0;
        this.addressCalcStatus.clear();
        $('#instructions').val('');
        $('#log').empty();
        this.reset();
    }
    
    updateDisplay() {
        // Clear any existing highlights when updating display
        this.clearHighlights();
        
        // Update clock
        $('#clock').text(this.clock);
        
        // Update instruction status table
        this.updateInstructionTable();
        
        // Update reservation station table
        this.updateReservationTable();
        
        // Update register table
        this.updateRegisterTable();
        
        // Update memory table
        this.updateMemoryTable();
    }
    
    updateInstructionTable() {
        const tbody = $('#instruction-body');
        tbody.empty();
        
        this.instructionStatus.forEach((status, index) => {
            const row = $('<tr>');
            
            // Create instruction cell with hover functionality
            const instructionCell = $('<td>');
            instructionCell.text(status.instruction);
            instructionCell.css('cursor', 'pointer');
            instructionCell.attr('data-instruction-index', index);
            
            // Add hover event listeners
            instructionCell.on('mouseenter', () => this.highlightInstruction(index));
            instructionCell.on('mouseleave', () => this.clearHighlights());
            
            row.append(instructionCell);
            row.append(`<td>${status.issue || '-'}</td>`);
            row.append(`<td>${status.execStart ? `${status.execStart}-${status.execEnd || this.clock}` : '-'}</td>`);
            row.append(`<td>${status.writeback || '-'}</td>`);
            
            const statusCell = $('<td>');
            statusCell.text(status.status);
            statusCell.addClass(`status-${status.status}`);
            row.append(statusCell);
            
            tbody.append(row);
        });
    }
    
    updateReservationTable() {
        const tbody = $('#reservation-body');
        tbody.empty();
        
        for (const [name, station] of Object.entries(this.reservationStations)) {
            const row = $('<tr>');
            
            // Create station name cell with hover functionality
            const nameCell = $('<td>');
            nameCell.text(name);
            nameCell.css('cursor', 'pointer');
            nameCell.attr('data-station', name);
            
            // Add hover event listeners
            nameCell.on('mouseenter', () => this.highlightStation(name));
            nameCell.on('mouseleave', () => this.clearHighlights());
            
            row.append(nameCell);
            
            const busyCell = $('<td>');
            busyCell.text(station.busy ? 'Yes' : 'No');
            busyCell.addClass(station.busy ? 'busy-true' : 'busy-false');
            row.append(busyCell);
            
            row.append(`<td>${station.op || '-'}</td>`);
            row.append(`<td>${station.vj || '-'}</td>`);
            row.append(`<td>${station.vk || '-'}</td>`);
            row.append(`<td>${station.qj || '-'}</td>`);
            row.append(`<td>${station.qk || '-'}</td>`);
            row.append(`<td>${station.a || '-'}</td>`);
            
            tbody.append(row);
        }
    }
    
    updateRegisterTable() {        
        // Remove any remaining placeholder cells
        $('.placeholder-cell').remove();

        // Update header
        const header = $('#register-header');
        header.empty();
        header.append('<th>Type</th>');
        
        for (let i = 0; i < REGISTER_COUNT; i++) {
            header.append(`<th>R${i}</th>`);
        }
        
        // Update Qi row
        const qiRow = $('#register-qi-row');
        qiRow.empty();
        qiRow.append('<td><strong>Qi</strong></td>');
        
        for (let i = 0; i < REGISTER_COUNT; i++) {
            const regName = `R${i}`;
            const reg = this.registers[regName];
            qiRow.append(`<td>${reg.qi || '-'}</td>`);
        }
        
        // Update value row
        const valueRow = $('#register-value-row');
        valueRow.empty();
        valueRow.append('<td><strong>Value</strong></td>');
        
        for (let i = 0; i < REGISTER_COUNT; i++) {
            const regName = `R${i}`;
            const reg = this.registers[regName];
            valueRow.append(`<td>${reg.value}</td>`);
        }
    }
    
    updateMemoryTable() {
        // Remove any remaining placeholder cells
        $('.placeholder-cell').remove();

        // Update header
        const header = $('#memory-header');
        header.empty();
        header.append('<th>Row</th>');
        
        // 16 columns, each representing an address offset
        for (let i = 0; i < 16; i++) {
            const offset = i * 4;
            header.append(`<th>+${offset}</th>`);
        }
        
        // Clear table body
        const tbody = $('#memory-body');
        tbody.empty();
        
        // 8 rows, each row 64 bytes (16 addresses)
        for (let row = 0; row < 8; row++) {
            const baseAddr = row * 64; // Starting address for each row
            const valueRow = $('<tr>');
            valueRow.append(`<td><strong>${baseAddr}</strong></td>`);
            
            // 16 addresses per row
            for (let col = 0; col < 16; col++) {
                const addr = baseAddr + col * 4;
                const value = this.memory[addr] || 0;
                valueRow.append(`<td>${value}</td>`);
            }
            
            tbody.append(valueRow);
        }
    }
    
    highlightStation(stationName) {
        // Clear any existing highlights first
        this.clearHighlights();
        
        // Highlight the station name cell
        $(`[data-station="${stationName}"]`).addClass('station-highlight');
        
        // Find and highlight corresponding instruction
        const instructionIndex = this.instructionStatus.findIndex(
            status => status.station === stationName
        );
        
        if (instructionIndex !== -1) {
            // Highlight only the instruction cell (first column)
            $(`[data-instruction-index="${instructionIndex}"]`).addClass('instruction-highlight');
        }
        
        // Highlight register cells that reference this station
        for (let i = 0; i < REGISTER_COUNT; i++) {
            const regName = `R${i}`;
            const reg = this.registers[regName];
            
            if (reg.qi === stationName) {
                // Highlight the Qi cell for this register
                $(`#register-qi-row td:eq(${i + 1})`).addClass('register-highlight');
            }
        }
    }
    
    clearHighlights() {
        // Remove all highlight classes
        $('.station-highlight').removeClass('station-highlight');
        $('.instruction-highlight').removeClass('instruction-highlight');
        $('.register-highlight').removeClass('register-highlight');
        $('.register-value-highlight').removeClass('register-value-highlight');
    }
    
    highlightInstruction(instructionIndex) {
        // Clear any existing highlights first
        this.clearHighlights();
        
        const status = this.instructionStatus[instructionIndex];
        if (!status) return;
        
        // Highlight the instruction cell (same style as station highlight)
        $(`[data-instruction-index="${instructionIndex}"]`).addClass('instruction-highlight');
        
        // Highlight corresponding reservation station
        if (status.station) {
            $(`[data-station="${status.station}"]`).addClass('station-highlight');
        }
        
        // Highlight related registers (operands and destination)
        const instruction = this.instructions[instructionIndex];
        if (instruction) {
            // Highlight destination register (Qi row)
            if (instruction.rd) {
                const regIndex = parseInt(instruction.rd.substring(1));
                $(`#register-qi-row td:eq(${regIndex + 1})`).addClass('register-highlight');
            }
            
            // Highlight source registers (Value row)
            if (instruction.rs) {
                const regIndex = parseInt(instruction.rs.substring(1));
                $(`#register-value-row td:eq(${regIndex + 1})`).addClass('register-value-highlight');
            }
            
            if (instruction.rt) {
                const regIndex = parseInt(instruction.rt.substring(1));
                $(`#register-value-row td:eq(${regIndex + 1})`).addClass('register-value-highlight');
            }
            
            // For load/store instructions, highlight base register (Value row)
            if (instruction.base) {
                const regIndex = parseInt(instruction.base.substring(1));
                $(`#register-value-row td:eq(${regIndex + 1})`).addClass('register-value-highlight');
            }
        }
    }
}

// Initialize simulator
$(document).ready(() => {
    window.simulator = new TomasuloSimulator();
    
    const exampleInstructions = 
`LD R1, 100(R0)
LD R2, 200(R0)
MUL R3, R1, R2
SUB R5, R2, R1
ADD R4, R3, R1
ST R4, 300(R5)
`;
    // Add example instructions
    $('#instructions').val(exampleInstructions);
});
