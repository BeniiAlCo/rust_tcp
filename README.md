rust_tcp

(Following the tutorial by Jon Gjengset, hosted at https://www.youtube.com/watch?v=bzja9fQWzdA)

TCP is one of the fundamental protocols of the internet.
It enables two hosts to talk to one another in a reliable way.
It puts in place certain guarentees about the data that is sent/recieved, such as the size, and order of the data.

The goal for this project is to implement something that can talk to a 'real server', for some definition of 'real' and 'server'.
That is, we want the ability to talk to some host that exists on the internet -- one that is not our own.

No advanced extensions;
No congestion control;
etc.

This implementation will follow RFC 793: https://www.rfc-editor.org/rfc/rfc793

For a tutorial on the basics of TCP/IP, use https://www.rfc-editor.org/rfc/rfc1180

RFC 7414 lists other relevant RFCs that may need to be implemented (here, the focus is on those listed under 'Core Functionality, in section 2 -- 793, 1122, 5681, 6093, 6298, 6691): https://www.rfc-editor.org/rfc/rfc7414

RFC 2525 lists a number of known implementation problems and there potential solutions: https://www.rfc-editor.org/rfc/rfc2525

Finally, RFC 2398 includes tools for testing an implementation: https://www.rfc-editor.org/rfc/rfc2398
