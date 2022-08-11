# Build and run the auction Leo program.
(
  cd ./project/examples/auction || exit
  $LEO run main
)

# Build and run the bubblesort Leo program.
(
  cd ./project/examples/bubblesort || exit
  $LEO run bubblesort
)

# Build and run the core example Leo program.
(
  cd ./project/examples/core || exit
  $LEO run main
)

# Build and run the groups example Leo program.
(
  cd ./project/examples/groups || exit
  $LEO run main
)

# Build and run the helloworld Leo program.
(
  cd ./project/examples/helloworld || exit
  $LEO run main
)

# Build and run the import point example Leo program.
(
  cd ./project/examples/import_point || exit
  $LEO run main
)

# Build and run the interest example Leo programs.
(
  cd ./project/examples/import_point || exit

  # Run the fixed period interest program.
  $LEO run fixed_period_interest

  # Run the bounded period interest program.
  $LEO run bounded_period_interest
)

# Build and run the message example Leo program.
(
  cd ./project/examples/message || exit
  $LEO run main
)

# Build and run the tic tac toe example Leo program.
(
  cd ./project/examples/tictactoe || exit
  $LEO run main
)

# Build and run the token example programs.
(
  cd ./project/examples/token || exit

  # Run the mint program.
  $LEO run mint

  # Run the transfer program.
  $LEO run transfer
)

# Build and run the hackers-delight/ntzdebruijin program.
(
  cd ./project/examples/hackers-delight/ntzdebruijin || exit
  $LEO run
)

# Build and run the hackers-delight/ntzgaudet program.
(
  cd ./project/examples/hackers-delight/ntzgaudet || exit
  $LEO run
)

# Build and run the hackers-delight/ntzloops program.
(
  cd ./project/examples/hackers-delight/ntzloops || exit
  $LEO run
)

# Build and run the hackers-delight/ntzmasks program.
(
  cd ./project/examples/hackers-delight/ntzmasks || exit
  $LEO run
)

# Build and run the hackers-delight/ntzreisers program.
(
  cd ./project/examples/hackers-delight/ntzreisers || exit
  $LEO run
)

# Build and run the hackers-delight/ntzseals program.
(
  cd ./project/examples/hackers-delight/ntzseals || exit
  $LEO run
)

# Build and run the hackers-delight/ntzsearchtree program.
(
  cd ./project/examples/hackers-delight/ntzsearchtree || exit
  $LEO run
)

# Build and run the hackers-delight/ntzsmallvals program.
(
  cd ./project/examples/hackers-delight/ntzsmallvals || exit
  $LEO run
)
