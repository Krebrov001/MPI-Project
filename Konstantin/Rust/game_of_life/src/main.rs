extern crate mpi;

use std::env;
use std::process;
use std::io::Read;
use std::io::Write;
use std::result::Result;
use std::io::Error;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::fs::OpenOptions; 
use mpi::traits::*;
use mpi::collective::SystemOperation;
use mpi::request::*;


fn read_in_file(filename : &String, grid_size : &mut i32, num_generations : &mut i32, output_generations : &mut i32, num_processors : i32) -> Vec<Vec<u8>>
{    
    /* Enforce input file naming conventions. */
    
    let input_as_text = "input".to_string();
    if (input_as_text != &filename[0..input_as_text.len()]) {
        eprintln!("ERROR: the file name {} needs to be in the form inputN, where N is an integer.", filename);
        eprintln!("Example of a valid file name: input2");
        process::abort();
    }
    
    let start : usize = input_as_text.len();
    let end   : usize = filename.len();
    let filename_number = &filename[start..end];
    
    if filename_number.is_empty() {
        eprintln!("ERROR: the file name {} needs to be in the form inputN, where N is an integer.", filename);
        eprintln!("Example of a valid file name: input2");
        process::abort();
    }
    
    for x in filename_number.chars() {
        if (!x.is_digit(10)) {
            eprintln!("ERROR: the file name {} needs to be in the form inputN, where N is an integer.", filename);
            eprintln!("Example of a valid file name: input2");
            process::abort();
        }
    }
    
    /* Read the contents of the file into memory. */
    
    // The open static method can be used to open a file in read-only mode.
    let mut file_handler = File::open(filename);
    // If File::open() succeeds, it returns an instance of Ok() that contains a file handler.
    // If File::open() fails, it returns an instance of Err() that contains more information about the kind of error that happened.
    let mut file_handler = match file_handler {
        Ok(file) => file,
        Err(error) => {
            eprintln!("ERROR:");
            eprintln!("{} could not be opened", filename);
            process::abort();
        }
    };
    let mut file_reader = BufReader::new(file_handler);
    
    // Read the first line in the file into memory as a String.
    let mut temp_string = String::new();
    file_reader.read_line(&mut temp_string);
    
    // Split the string into tokens, and assign each token to the corresponding parameter, if possible.
    let tokens : Vec<&str> = temp_string.split_whitespace().collect();
    
    // Try to assign the first token to grid_size.
    let mut curr_token = match tokens.get(0) {
        Some(x) => (x),
        None    => ""
    };
    *grid_size = match curr_token.parse::<i32>() {
        Ok(i)  => i,
        Err(e) => -1
    };
    
    // Try to assign the second token to num_generations.
    curr_token = match tokens.get(1) {
        Some(x) => (x),
        None    => ""
    };
    *num_generations = match curr_token.parse::<i32>() {
        Ok(i)  => i,
        Err(e) => -1
    };
    
    // Try to assign the third token to output_generations.
    curr_token = match tokens.get(2) {
        Some(x) => (x),
        None    => ""
    };
    *output_generations = match curr_token.parse::<i32>() {
        Ok(i)  => i,
        Err(e) => -1
    };
    
    // Check if reading the parameters from the file succeeded.
    if (*grid_size == -1 || *num_generations == -1 || *output_generations == -1) {
        eprintln!("ERROR: {} could not be read successfully", filename);
        eprintln!("File format:\n\
        N G O\n\
        NxN grid\n\
        \n\
        N - the size of the NxN grid of 0s and 1s\n\
        G - the number of generations to iterate through\n\
        O - the output generation value"
        );
        process::abort();
    }
    
    /* Enforce constraints on the input parameters. */
    
    if *grid_size <= 0 {
        eprintln!("ERROR:");
        eprintln!("N - the size of the NxN grid must be > 0");
        process::abort();
    }
    
    if *grid_size % 8 != 0 {
        eprintln!("ERROR:");
        eprintln!("N - the size of the NxN grid must be divisible by 8");
        process::abort();
    }
    
    if *num_generations <= 0 {
        eprintln!("ERROR:");
        eprintln!("G - the number of generations to iterate through must be > 0");
        process::abort();
    }
    
    if *output_generations <= 0 {
        eprintln!("ERROR:");
        eprintln!("O - the output generation value must be > 0");
        process::abort();
    }
    
    if (*num_generations % *output_generations != 0) {
        eprintln!("ERROR:");
        eprintln!("G - the number of generations to iterate through must be divisible by O - the output generation value");
        process::abort();
    }
    
    // You need to validate that the number of processors your code is being run with evenly divides the N or size of your grid.
    if (*grid_size % num_processors != 0) {
        eprintln!("ERROR:");
        eprintln!("N - the size of the NxN grid must be divisible by the number of processors {}", num_processors);
        process::abort();
    }
    
    // In vec!() the first parameter is the initial value, the second parameter is the number of elements.
    let mut map : Vec<Vec<u8>> = vec![ vec![0; *grid_size as usize]; *grid_size as usize ];
    
    // Read the grid into memory.
    // https://stackoverflow.com/a/29582998/5500589
    
    // Loop through all the lines.
    for (row, line) in file_reader.lines().enumerate() {
        let line_text = line.unwrap();
        // Loop through the characters in each line.
        let mut col : usize = 0;
        for digit_char in line_text.chars() {
            map[row][col] = digit_char as u8;
            // NOTE: The ++ and -- operators are not supported in Rust!
            col += 1;
        }
    }
    
    // (*grid_size as usize) is necessary in order for the range have a type of Range<usize>
    // in order for the row and col indexes to have a type of usize.
    // All indexes in Rust must have a type of usize.
    for row in 0..(*grid_size as usize) {
        for col in 0..(*grid_size as usize) {
            map[row][col] -= '0' as u8;  // convert from char to int
        }
    }
    
    // In Rust a File is automatically closed once the scope of its owner ends.
    
    return map;
}


fn setCell(former_cell : u8, neighbors : u32) -> u8
{
    if former_cell == 1 {
        if neighbors < 2 || neighbors > 3 {
            return 0;
        } else {
            return 1;
        }
    } else {  // former_cell == 0
        if neighbors == 3 {
            return 1;
        } else {
            return 0;
        }
    }
}


fn main() {
    // Because the mpirun executable itself generates standard error and standard output,
    // I need to create a new File where the application's output should be written.
    let mut output_file_handler : Option<File>;
    // Similarly, this is the File where the application's timing information shoudl be written.
    let mut timing_file_handler : Option<File>;
    
    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    
    let comm_size : i32 = world.size();
    let my_rank   : i32 = world.rank();
    
    let argv : Vec<String> = env::args().collect();
    let argc : usize = env::args().len();
    
    let mut grid : Vec<u8>;
    
    if argc != 2 {
        // If incorrect arguments supplied, have rank 0 abort all the processes.
        // You always want to make sure that only one rank aborts all the processes.
        // The other processes do recieve::<i32>() for a message that never comes, so they are blocked.
        // If I would have put process::abort() inside the other processes as well, one of them could
        // potentially kill rank 0 before it had a chance to eprintln!() the message out to the screen.
        // A single process::abort() terminates the entire mpi system and therefore all the processes.
        // The blocking recieve is used to stop the other processes from running ahead and potentially
        // printing some extraneous outputs.
        if my_rank == 0 {
            // eprintln!() prints to the standard error
            eprintln!("Usage:\n$ {} <input_file>", argv[0]);
            process::abort();
        } else {
            // recieve::<i32>() returns a tuple `(i32, mpi::point_to_point::Status)`
            let __dummy = (world.process_at_rank(0).receive::<i32>()).0;
        }
    }
    
    let input_filename : &String = &argv[1];
    
    let mut grid_size : i32 = -1;
    let mut num_generations : i32 = -1;
    let mut output_generations : i32 = -1;

    if my_rank == 0 {
        // read in the file
        let temp_grid : Vec<Vec<u8>> = read_in_file(input_filename, &mut grid_size, &mut num_generations, &mut output_generations, comm_size);
        grid = vec![0; (grid_size * grid_size) as usize];
        
        // Copy the data from the temp_grid into the grid.
        for i in 0..grid_size {
            for j in 0..grid_size {
                grid[(i * grid_size + j) as usize] = temp_grid[i as usize][j as usize];
            }
        }
        
        // Send the data to all processes.
        for i in 0..comm_size {
            // buffered_send_with_tag::<i32>() takes the buffer (message) to send and the tag as parameters.
            world.process_at_rank(i).buffered_send_with_tag::<i32>(&grid_size, 0);  // tag 0 is for grid_size
            world.process_at_rank(i).buffered_send_with_tag::<i32>(&num_generations, 1);  // tag 1 is for num_generations
            world.process_at_rank(i).buffered_send_with_tag::<i32>(&output_generations, 2);  // tag 2 is for output_generations
        }
    } else {
        // Recieving the data is used as a synchronization mechanism for the other processes.
        // Rank 0 reads in the file, which takes a lot of time. I do not want my other processes to run off and leave
        // rank 0 behind. By doing the recieve, they are patiently waiting with the open mouths until rank 0 finally
        // finishes reading in the file and sends them their data.
        //
        // I am using a tagging system to distinguish the data. Rank 0 uses buffered_send_with_tag::<i32>() to send the data.
        // There is a slight chance that the messages could be recieved in a different order than they are sent.
        // If that is the case, explicitly provide tags to recieve the messages in the same order that they were sent.
        // There is no efficiency loss to this, but it prevents some bugs that might be caused when using the same tag
        // for all your sent and recieved data.
        
        // recieve_with_tag::<i32>() returns a tuple `(i32, mpi::point_to_point::Status)`
        // recieve_with_tag::<i32>() takes the tag as a parameter.
        grid_size = (world.process_at_rank(0).receive_with_tag::<i32>(0)).0;  // tag 0 is for grid_size
        num_generations = (world.process_at_rank(0).receive_with_tag::<i32>(1)).0;  // tag 1 is for num_generations
        output_generations = (world.process_at_rank(0).receive_with_tag::<i32>(2)).0;  // tag 2 is for output_generations
        
        //grid = vec![ vec![0; grid_size as usize]; grid_size as usize ];
        grid = vec![0; (grid_size * grid_size) as usize];
    }
    
    // Open up the output file.
    if my_rank == 0 {
        // This code creates the output filename based on the input filename.
        // input_filename    output_filename
        // input1            output1
        // input11           output11
        //
        // I can assume that the input_filename matches a certain format, because it was
        // already checked for that in the function read_in_file().
        let input_as_text = "input".to_string();
        let start : usize = input_as_text.len();
        let end   : usize = input_filename.len();
        let filename_number = &input_filename[start..end];
        let output_filename : String = "output".to_string() + filename_number;
        
        // The create() static method opens a file in write-only mode.
        // If the file already existed, the old content is destroyed. Otherwise, a new file is created.
        let create_file_handler = File::create(&output_filename);
        // If File::create() succeeds, it returns an instance of Ok() that contains a file handler.
        // If File::create() fails, it returns an instance of Err() that contains more information about the kind of error that happened.
        output_file_handler = match create_file_handler {
            Ok(file) => {
                let dummy : i8 = 5;
                // Send the success message to all processes.
                for i in 0..comm_size {
                    world.process_at_rank(i).send_with_tag::<i8>(&dummy, 100);
                }
                
                Some(file)  // quantity without a ; at the end, is returned by the match statement
            },
            
            Err(error) => {
                eprintln!("ERROR: Could not open the file {} for writing output.", output_filename);
                process::abort();
            }
        };
        
    // Make the other ranks either wait for a success message to be sent, or be killed upon failure.
    // This is used as a synchronization device.
    } else {
        // recieve_with_tag::<i8>() returns a tuple `(i8, mpi::point_to_point::Status)`
        // recieve_with_tag::<i8>() takes the tag as a parameter.
        let __dummy = (world.process_at_rank(0).receive_with_tag::<i8>(100)).0;
        output_file_handler = None;
    }
    
    /*
    if my_rank == 0 {
        output_file_handler.unwrap().write(b"Yes! It works!");
    }
    */
    
    // Open up the timing file.
    if my_rank == 0 {
        // This code creates the timing filename based on the input filename.
        // input_filename    timing_filename
        // input1            timing1
        // input11           timing11
        //
        // I can assume that the input_filename matches a certain format, because it was
        // already checked for that in the function read_in_file().
        let input_as_text = "input".to_string();
        let start : usize = input_as_text.len();
        let end   : usize = input_filename.len();
        let filename_number = &input_filename[start..end];
        let timing_filename : String = "timing".to_string() + filename_number;
        
        // The append() static method Sets the option for the append mode of file.
        //let create_file_handler = File::append(&timing_filename);
        let create_file_handler = OpenOptions::new().create(true).append(true).open(&timing_filename);
        // If it succeeds, it returns an instance of Ok() that contains a file handler.
        // If it fails, it returns an instance of Err() that contains more information about the kind of error that happened.
        timing_file_handler = match create_file_handler {
            Ok(file) => {
                let dummy : i8 = 5;
                // Send the success message to all processes.
                for i in 0..comm_size {
                    world.process_at_rank(i).send_with_tag::<i8>(&dummy, 100);
                }
                
                Some(file)  // quantity without a ; at the end, is returned by the match statement
            },
            
            Err(error) => {
                eprintln!("ERROR: Could not open the file {} for writing timing information.", timing_filename);
                process::abort();
            }
        };
        
    // Make the other ranks either wait for a success message to be sent, or be killed upon failure.
    // This is used as a synchronization device.
    } else {
        // recieve_with_tag::<i8>() returns a tuple `(i8, mpi::point_to_point::Status)`
        // recieve_with_tag::<i8>() takes the tag as a parameter.
        let __dummy = (world.process_at_rank(0).receive_with_tag::<i8>(100)).0;
        timing_file_handler = None;
    }
    
    // These variables are relevant for timing information.
    // Each rank gets it's own copy of these variables,
    // but only the local ones are relevant to a single rank.
    // And elapsed : f64 is only relevant to rank 0.
    // It contains the total elapsed time for the whole entire MPI program,
    // including the running time of all the ranks.
    
    let mut local_start   : f64 = 0.0;
    let mut local_finish  : f64 = 0.0;
    let mut local_elapsed : f64 = 0.0;
    let mut elapsed       : f64 = 0.0;
    
    // Makes all the ranks pause and wait for each other before continuing down.
    world.barrier();
    
    // Start the timer in each of the ranks.
    local_start = mpi::time();
    
    // Setup the chunks.
    let chunk_size : i32 = grid_size * grid_size / comm_size;  // the number of elements in a chunk
    let num_rows : i32 = chunk_size / grid_size;  // the number of rows in a chunk
    // In vec!() the first parameter is the initial value, the second parameter is the number of elements.
    
    //let mut chunk : Vec<Vec<u8>> = vec![ vec![0; grid_size as usize]; num_rows as usize ];
    //let mut chunk2 : Vec<Vec<u8>> = vec![ vec![0; grid_size as usize]; num_rows as usize ];
    
    // Make the vectors contiguous memory locations, in order to facilitate scattering.
    let mut chunk : Vec<u8> = vec![0; (grid_size * num_rows) as usize];
    let mut chunk2 : Vec<u8> = vec![0; (grid_size * num_rows) as usize];
    
    // Option<> either holds a value or it holds None,
    // similar in concept to a NULL pointer in C.
    let mut halo_above : Option<Vec<u8>>;
    let mut halo_below : Option<Vec<u8>>;
    
    // Setup the halos.
    if my_rank != 0 {
        halo_above = Some(vec![0; grid_size as usize]);
    } else {
        halo_above = None;
    }
    
    if my_rank != comm_size - 1 {
        halo_below = Some(vec![0; grid_size as usize]);
    } else {
        halo_below = None;
    }
    
    // Scatter from grid into chunk in each proccess.
    if my_rank == 0 {
        world.process_at_rank(0).scatter_into_root(grid.as_slice(), chunk.as_mut_slice());
    } else {
        world.process_at_rank(0).scatter_into(chunk.as_mut_slice());
    }
    
    // Copy the scattered data from the chunk into chunk2.
    for i in 0..(grid_size * num_rows) as usize {
        chunk2[i] = chunk[i];
    }
    
    
    let mut current_chunk : Option<&Vec<u8>>;
    let mut former_chunk  : Option<&Vec<u8>>;
        
    let mut row_above : Option<&Vec<u8>>;
    let mut row_below : Option<&Vec<u8>>;
    
    for generation in 0..num_generations {
        if (generation % 2 == 0) {
            current_chunk = Some(&chunk2);
            former_chunk = Some(&chunk);
        } else {
            current_chunk = Some(&chunk);
            former_chunk = Some(&chunk2);
        }
        
        /* The halo arrays get updated each iteration. */
        
        mpi::request::scope(|scope| {
            // If you have the below buffer.
            let request1 = match &halo_below {
                Some(x) => {
                    let start : usize = ((num_rows-1) * grid_size) as usize;
                    let end   : usize = ((num_rows-1) * grid_size + grid_size) as usize;
                    // Send the bottom row in the chunk to the rank directly below you.
                    Some(world.process_at_rank(my_rank+1).immediate_send(scope, &former_chunk.unwrap()[start..end]))
                }
                None => { None }
            };
            
            // If you have the above buffer.
            let request2 = match &halo_above {
                Some(x) => {
                    let start : usize = 0;
                    let end   : usize = grid_size as usize;
                    // Send the top row in the chunk to the rank directly above you.
                    Some(world.process_at_rank(my_rank-1).immediate_send(scope, &former_chunk.unwrap()[start..end]))
                }
                None => { None }
            };
            
            // If you have the below buffer.
            halo_below = match &halo_below {
                Some(x) => {
                    // Recieve the top row in the chunk from the rank directly below you.
                    Some((world.process_at_rank(my_rank+1).receive_vec::<u8>()).0)
                }
                None => { None }
            };
            
            // If you have the above buffer.
            halo_above = match &halo_above {
                Some(x) => {
                    // Recieve the bottom row in the chunk from the rank directly above you.
                    Some((world.process_at_rank(my_rank-1).receive_vec::<u8>()).0)
                }
                None => { None }
            };
            
            // If you have the below buffer.
            match &halo_below {
                Some(x) => {
                    request1.unwrap().wait_without_status();
                }
                None => {}
            };
            
            // If you have the above buffer.
            match &halo_above {
                Some(x) => {
                    request2.unwrap().wait_without_status();
                }
                None => {}
            };
        });
    
        // Process the data.
        // This loops through all the rows.
        // In a single iteration of this loop, an entire row of cells, with all the columns, is computed.
        // Before performing the actual Game of Life Algorithm, the rows above and below that cell are pre-computed
        // in order to make it easier.
        for row in 0..num_rows {
            if my_rank == 0 {  // top rank
                // determine what the row above should be
                if row == 0 {
                    row_above = None;
                } else {
                    let start : usize = ((row-1) * grid_size) as usize;
                    let end   : usize = ((row-1) * grid_size + grid_size) as usize;
                    row_above = Some(&former_chunk.unwrap()[start..end].to_vec());
                }
                // determine what the row below should be
                if row == num_rows-1 {
                    //row_below = Some(&halo_below);
                    row_below = match &halo_below {
                        Some(x) => Some(x),
                        None => None
                    };
                } else {
                    let start : usize = ((row+1) * grid_size) as usize;
                    let end   : usize = ((row+1) * grid_size + grid_size) as usize;
                    row_below = Some(&former_chunk.unwrap()[start..end].to_vec());
                }
            } else if my_rank == comm_size-1 {  // bottom rank
                // determine what the row above should be
                if row == 0 {
                    //row_above = Some(&halo_above.unwrap());
                    row_above = match &halo_above {
                        Some(x) => Some(x),
                        None => None
                    };
                } else {
                    let start : usize = ((row-1) * grid_size) as usize;
                    let end   : usize = ((row-1) * grid_size + grid_size) as usize;
                    row_above = Some(&former_chunk.unwrap()[start..end].to_vec());
                }
                // determine what the row below should be
                if row == num_rows-1 {
                    row_below = None;
                } else {
                    let start : usize = ((row+1) * grid_size) as usize;
                    let end   : usize = ((row+1) * grid_size + grid_size) as usize;
                    row_below = Some(&former_chunk.unwrap()[start..end].to_vec());
                }
            } else {  // middle rank
                // determine what the row above should be
                if row == 0 {
                    //row_above = Some(&halo_above.unwrap());
                    row_above = match &halo_above {
                        Some(x) => Some(x),
                        None => None
                    };
                } else {
                    let start : usize = ((row-1) * grid_size) as usize;
                    let end   : usize = ((row-1) * grid_size + grid_size) as usize;
                    row_above = Some(&former_chunk.unwrap()[start..end].to_vec());
                }
                // determine what the row below should be
                if row == num_rows-1 {
                    //row_below = Some(&halo_below.unwrap());
                    row_below = match &halo_below {
                        Some(x) => Some(x),
                        None => None
                    };
                } else {
                    let start : usize = ((row+1) * grid_size) as usize;
                    let end   : usize = ((row+1) * grid_size + grid_size) as usize;
                    row_below = Some(&former_chunk.unwrap()[start..end].to_vec());
                }
            }
            
            let mut neighbors : u32 = 0;
            // left 
            neighbors += (former_chunk.unwrap()[(row * grid_size + 1) as usize]) as u32;
            match &row_above {
                Some(row_above_vector) => {
                    neighbors += (row_above_vector[0] + row_above_vector[1]) as u32;
                },
                None => {}
            };
            match &row_below {
                Some(row_below_vector) => {
                    neighbors += (row_below_vector[0] + row_below_vector[1]) as u32;
                },
                None => {}
            };
            current_chunk.unwrap()[(row * grid_size) as usize] = setCell(former_chunk.unwrap()[(row * grid_size) as usize], neighbors);
        }
        
    }
    
    
    
    
    // Stop the timer in each of the ranks.
    local_finish = mpi::time();
    // Calculate the elapsed time in each of the ranks.
    local_elapsed = local_finish - local_start;
    // The global elapsed time (the time for the whole program) is the time it took the "slowest" process to finish,
    // the maximum local_elapsed time.
    // Actually compute the reduce, by having two separate function calls.
    if my_rank == 0 {
        world.process_at_rank(0).reduce_into_root(&local_elapsed, &mut elapsed, SystemOperation::max());
    } else {
        world.process_at_rank(0).reduce_into(&local_elapsed, SystemOperation::max());
    }
    
    if my_rank == 0 {
        // The timing information is apended to the end of the file.
        // f64.to_string() produces String ; String[..] produces str ; str.as_bytes() produces &[u8]
        (&timing_file_handler).as_ref().unwrap().write(elapsed.to_string()[..].as_bytes());
        (&timing_file_handler).as_ref().unwrap().write(b"\n");
        (&timing_file_handler).as_ref().unwrap().flush();
    }
    
    return;
}
